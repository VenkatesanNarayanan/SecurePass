use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, KeyInit};
use chacha20poly1305::aead::Aead;
use aead::Payload;
use argon2::{Argon2, Params, Algorithm, Version};
use rand::RngCore;
use rand::rngs::OsRng;

/// Cryptographic manager for SecurePass encryption operations
pub struct CryptoManager {
    key: Key,
}

impl CryptoManager {
    /// Derive a 256-bit key from master password using Argon2id
    pub fn derive_key(master_password: &str, salt: &[u8; 32]) -> Result<Key, Box<dyn std::error::Error>> {
        println!("Deriving key with password length: {}, salt length: {}", master_password.len(), salt.len());
        let mut key = Key::default();
        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(65536, 3, 1, None).unwrap(), // 64 KiB, 3 iterations, 1 thread
        );

        println!("Starting Argon2 hashing...");
        match argon2.hash_password_into(master_password.as_bytes(), salt, &mut key) {
            Ok(_) => {
                println!("Argon2 hashing completed successfully");
                Ok(key)
            },
            Err(e) => {
                println!("Argon2 hashing failed: {:?}", e);
                Err(format!("Argon2 hashing failed: {:?}", e).into())
            }
        }
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