use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, KeyInit};
use chacha20poly1305::aead::Aead;
use argon2::{Argon2, Params, Algorithm, Version};
use rand::RngCore;
use rand::rngs::OsRng;

/// Argon2id cost parameters used for every vault.
///
/// These are NOT stored in the vault file, so changing any of them makes
/// every existing vault (local, `.spvault` backups and the Drive copy)
/// impossible to unlock. `test_key_derivation_is_stable` pins them; a change
/// needs a versioned vault header and a migration first.
///
/// `argon2`'s memory cost is in KiB blocks: 65_536 KiB = 64 MiB.
/// 64 MiB / 3 passes is RFC 9106's second recommended setting (with 1 lane
/// instead of 4) and above OWASP's Argon2id minimums.
pub const ARGON2_MEMORY_KIB: u32 = 64 * 1024;
pub const ARGON2_ITERATIONS: u32 = 3;
pub const ARGON2_PARALLELISM: u32 = 1;

/// Cryptographic manager for SecurePass encryption operations
pub struct CryptoManager {
    key: Key,
}

impl CryptoManager {
    /// Derive a 256-bit key from master password using Argon2id
    pub fn derive_key(master_password: &str, salt: &[u8; 32]) -> Result<Key, Box<dyn std::error::Error>> {
        let mut key = Key::default();
        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(ARGON2_MEMORY_KIB, ARGON2_ITERATIONS, ARGON2_PARALLELISM, None)
                .map_err(|e| format!("Invalid Argon2 parameters: {}", e))?,
        );

        argon2.hash_password_into(master_password.as_bytes(), salt, &mut key)
            .map_err(|e| format!("Key derivation failed: {}", e))?;
        Ok(key)
    }

    /// Create a new CryptoManager with the derived key
    pub fn new(master_password: &str, salt: &[u8; 32]) -> Result<Self, Box<dyn std::error::Error>> {
        let key = Self::derive_key(master_password, salt)?;
        Ok(Self { key })
    }

    /// Generate a random 32-byte salt for key derivation
    pub fn generate_salt() -> [u8; 32] {
        let mut salt = [0u8; 32];
        OsRng.fill_bytes(&mut salt);
        salt
    }

    /// Generate a random 12-byte nonce for ChaCha20-Poly1305
    pub fn generate_nonce() -> Nonce {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        Nonce::from(nonce_bytes)
    }

    /// Encrypt plaintext using ChaCha20-Poly1305 with AAD
    pub fn encrypt(&self, plaintext: &[u8], nonce: &Nonce, aad: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let cipher = ChaCha20Poly1305::new(&self.key);
        let ciphertext = cipher.encrypt(nonce, aead::Payload { msg: plaintext, aad })
            .map_err(|e| format!("Encryption failed: {}", e))?;
        Ok(ciphertext)
    }

    /// Decrypt ciphertext using ChaCha20-Poly1305 with AAD
    pub fn decrypt(&self, ciphertext: &[u8], nonce: &Nonce, aad: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let cipher = ChaCha20Poly1305::new(&self.key);
        let plaintext = cipher.decrypt(nonce, aead::Payload { msg: ciphertext, aad })
            .map_err(|e| format!("Decryption failed: {}", e))?;
        Ok(plaintext)
    }

    /// Extract raw key bytes so the key can be used across async boundaries.
    pub fn get_key_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(self.key.as_slice());
        bytes
    }

    /// Reconstruct a CryptoManager from raw key bytes (no Argon2 re-derivation).
    pub fn from_key_bytes(bytes: &[u8; 32]) -> Self {
        Self { key: Key::from(*bytes) }
    }

    /// Securely zero out memory (defense against memory dumps)
    pub fn zeroize_key(&mut self) {
        self.key = Key::default();
    }
}

/// Generate AAD (Additional Authenticated Data) as specified in requirements
/// AAD includes: Entry ID, Timestamp, Category
pub fn generate_aad(entry_id: &str, timestamp: i64, category: &str) -> Vec<u8> {
    format!("{}:{}:{}", entry_id, timestamp, category).into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_derivation() {
        let password = "test_password";
        let salt = CryptoManager::generate_salt();

        let key1 = CryptoManager::derive_key(password, &salt).unwrap();
        let key2 = CryptoManager::derive_key(password, &salt).unwrap();

        assert_eq!(key1, key2, "Same password and salt should produce same key");
    }

    /// Known-answer test: a fixed password and salt must always give this key.
    /// If it fails, the Argon2 parameters or algorithm changed, and existing
    /// vaults would no longer unlock. See the ARGON2_* constants.
    #[test]
    fn test_key_derivation_is_stable() {
        let salt = [7u8; 32];
        let key = CryptoManager::derive_key("correct horse battery staple", &salt).unwrap();
        let hex: String = key.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, EXPECTED_KAT_KEY_HEX, "Argon2 key derivation changed — existing vaults would not unlock");
    }

    // Produced by Argon2id v0x13, 64 MiB, 3 iterations, 1 lane (the settings
    // every vault since v1.0 was created with).
    const EXPECTED_KAT_KEY_HEX: &str = "a946e7002480e5e1a79524aee83439e85f78bbe4837183d7b66ed45adcd48261";

    #[test]
    fn test_encrypt_decrypt() {
        let password = "test_password";
        let salt = CryptoManager::generate_salt();
        let crypto = CryptoManager::new(password, &salt).unwrap();

        let plaintext = b"Hello, SecurePass!";
        let nonce = CryptoManager::generate_nonce();
        let aad = b"test_aad";

        let ciphertext = crypto.encrypt(plaintext, &nonce, aad).unwrap();
        let decrypted = crypto.decrypt(&ciphertext, &nonce, aad).unwrap();

        assert_eq!(plaintext, decrypted.as_slice(), "Decryption should recover original plaintext");
    }

    #[test]
    fn test_wrong_aad_fails() {
        let password = "test_password";
        let salt = CryptoManager::generate_salt();
        let crypto = CryptoManager::new(password, &salt).unwrap();

        let plaintext = b"Hello, SecurePass!";
        let nonce = CryptoManager::generate_nonce();

        let ciphertext = crypto.encrypt(plaintext, &nonce, b"correct_aad").unwrap();
        let result = crypto.decrypt(&ciphertext, &nonce, b"wrong_aad");

        assert!(result.is_err(), "Decryption with wrong AAD should fail");
    }
}