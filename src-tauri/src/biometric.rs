//! Biometric unlock: fingerprint on Android, Windows Hello on Windows.
//!
//! The 32-byte vault key — never the master password — is protected by a key
//! the operating system only releases after biometric verification, and the
//! result is stored in `biometric_unlock.json` next to `vault.dat`. The
//! decrypted vault key stays in Rust; it is never sent to the web UI.
//!
//! - Android: the vault key is encrypted with an AES key in the Android
//!   Keystore that requires strong biometrics for every use
//!   (`tauri-plugin-biometric`'s `biometric_cipher`: BiometricPrompt +
//!   CryptoObject). Fingerprint only; the phone's PIN is not accepted.
//! - Windows: a Windows Hello key credential (TPM-backed where available)
//!   signs a random challenge; Hello's RSA signatures are deterministic, so
//!   SHA-256 of the signature is a stable wrapping key for the vault key
//!   (ChaCha20-Poly1305). Each unlock needs Hello verification. Windows Hello
//!   always also accepts the Windows Hello PIN; that can't be turned off.
//!
//! Shared rules, enforced here for every platform:
//! - the master password is required again once [`PASSWORD_INTERVAL_SECS`]
//!   have passed since the last password unlock;
//! - the enrolment is dropped when it no longer opens this vault (vault salt
//!   or key changed: master password changed here or on another device, vault
//!   restored or re-created; those commands also call [`clear`]) or when the
//!   platform key is gone or invalidated (Android: a fingerprint was added;
//!   Windows: the Hello credential was removed or replaced).

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The master password must be typed at least this often (7 days).
pub const PASSWORD_INTERVAL_SECS: i64 = 7 * 24 * 60 * 60;

/// Why a biometric unlock didn't produce a key.
#[derive(Debug, PartialEq)]
#[cfg_attr(not(any(target_os = "android", windows)), allow(dead_code))]
pub enum UnlockError {
    /// The user dismissed the prompt (or the system did); nothing changes.
    Cancelled,
    /// The periodic master-password check is due; the enrolment is kept.
    PasswordRequired,
    /// The enrolment can't be used any more and has been deleted.
    Invalidated(String),
    /// Anything else (too many attempts, hardware unavailable, I/O ...).
    Failed(String),
}

/// What the UI should call the feature on this platform.
pub fn kind() -> &'static str {
    if cfg!(windows) {
        "windowsHello"
    } else {
        "fingerprint"
    }
}

const FILE_NAME: &str = "biometric_unlock.json";

#[derive(Serialize, Deserialize)]
struct Enrollment {
    version: u32,
    /// The protected vault key, in the platform backend's own format.
    data: String,
    /// Salt of the vault the key belongs to (base64), to spot a re-keyed vault.
    vault_salt: String,
    /// Unix time of the last unlock with the master password.
    last_password_unlock: i64,
}

fn path(app: &tauri::AppHandle) -> PathBuf {
    crate::commands::get_vault_path(app).with_file_name(FILE_NAME)
}

fn read(app: &tauri::AppHandle) -> Option<Enrollment> {
    let data = std::fs::read_to_string(path(app)).ok()?;
    serde_json::from_str(&data).ok()
}

fn write(app: &tauri::AppHandle, enrollment: &Enrollment) -> Result<(), String> {
    let data = serde_json::to_string(enrollment).map_err(|e| e.to_string())?;
    std::fs::write(path(app), data).map_err(|e| e.to_string())
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Biometrics are set up and usable on this device.
pub fn is_available(app: &tauri::AppHandle) -> bool {
    backend::is_available(app)
}

pub fn is_enrolled(app: &tauri::AppHandle) -> bool {
    read(app).is_some()
}

/// Forget the enrolment (the platform key itself is reused next time).
pub fn clear(app: &tauri::AppHandle) {
    let _ = std::fs::remove_file(path(app));
}

/// Restart the periodic master-password clock after a password unlock.
pub fn note_password_unlock(app: &tauri::AppHandle) {
    if let Some(mut e) = read(app) {
        e.last_password_unlock = now();
        let _ = write(app, &e);
    }
}

/// Verify the user biometrically and store `key`, protected by the platform.
pub fn enroll(app: &tauri::AppHandle, key: &[u8; 32], vault_salt: &[u8; 32]) -> Result<(), String> {
    let data = backend::protect(app, key)?;
    write(
        app,
        &Enrollment {
            version: 1,
            data,
            vault_salt: B64.encode(vault_salt),
            last_password_unlock: now(),
        },
    )
}

/// Verify the user biometrically and return the vault key for the vault
/// whose file currently has `vault_salt`.
pub fn unlock_key(app: &tauri::AppHandle, vault_salt: &[u8; 32]) -> Result<[u8; 32], UnlockError> {
    let enrollment = read(app)
        .ok_or_else(|| UnlockError::Invalidated("Biometric unlock isn't set up.".into()))?;

    if enrollment.vault_salt != B64.encode(vault_salt) {
        clear(app);
        return Err(UnlockError::Invalidated(
            "The vault was re-encrypted (new master password or restore). Unlock with your master password, then turn biometric unlock on again in Settings.".into(),
        ));
    }
    if now() - enrollment.last_password_unlock > PASSWORD_INTERVAL_SECS {
        return Err(UnlockError::PasswordRequired);
    }

    backend::unprotect(app, &enrollment.data).inspect_err(|e| {
        if matches!(e, UnlockError::Invalidated(_)) {
            clear(app);
        }
    })
}

/// Overwrite a buffer before it is dropped (best effort).
#[cfg_attr(not(any(target_os = "android", windows)), allow(dead_code))]
fn wipe(bytes: &mut [u8]) {
    bytes.iter_mut().for_each(|b| *b = 0);
    std::hint::black_box(&bytes);
}

/// Decode a base64 vault key into exactly 32 bytes, wiping the temporaries.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
fn key_from_b64(encoded: &str) -> Option<[u8; 32]> {
    let mut bytes = B64.decode(encoded.as_bytes()).ok()?;
    let key = <[u8; 32]>::try_from(bytes.as_slice()).ok();
    wipe(&mut bytes);
    key
}

// ── Android: Keystore key bound to strong biometrics ─────────────────────────

#[cfg(target_os = "android")]
mod backend {
    use super::{key_from_b64, wipe, UnlockError, B64};
    use base64::Engine;
    use tauri_plugin_biometric::{AuthOptions, BiometricExt};

    fn options(title: &str, subtitle: &str) -> AuthOptions {
        AuthOptions {
            // Fingerprint only: the phone's PIN must not open the vault.
            allow_device_credential: false,
            cancel_title: Some("Use master password".into()),
            title: Some(title.into()),
            subtitle: Some(subtitle.into()),
            confirmation_required: Some(false),
            ..Default::default()
        }
    }

    pub fn is_available(app: &tauri::AppHandle) -> bool {
        app.biometric().status().map(|s| s.is_available).unwrap_or(false)
    }

    pub fn protect(app: &tauri::AppHandle, key: &[u8; 32]) -> Result<String, String> {
        let mut opts = options("Turn on fingerprint unlock", "Confirm with your fingerprint");
        opts.data_to_encrypt = Some(B64.encode(key));
        app.biometric()
            .biometric_cipher("Turn on fingerprint unlock for SecurePass".into(), opts)
            .map(|r| r.data)
            .map_err(|e| match classify(&e) {
                UnlockError::Cancelled => "Fingerprint setup was cancelled.".to_string(),
                UnlockError::Invalidated(m) | UnlockError::Failed(m) => m,
                UnlockError::PasswordRequired => unreachable!(),
            })
    }

    pub fn unprotect(app: &tauri::AppHandle, data: &str) -> Result<[u8; 32], UnlockError> {
        let mut opts = options("Unlock SecurePass", "Use your fingerprint to open your vault");
        opts.data_to_decrypt = Some(data.to_string());
        let result = app
            .biometric()
            .biometric_cipher("Unlock SecurePass".into(), opts)
            .map_err(|e| classify(&e))?;
        let key = key_from_b64(&result.data);
        // The plugin's JVM side keeps its own immutable copy until GC.
        wipe(&mut result.data.into_bytes());
        key.ok_or_else(|| UnlockError::Invalidated("The stored fingerprint key was unreadable.".into()))
    }

    fn classify(e: &tauri_plugin_biometric::Error) -> UnlockError {
        use tauri::plugin::mobile::PluginInvokeError;
        let tauri_plugin_biometric::Error::PluginInvoke(PluginInvokeError::InvokeRejected(resp)) = e else {
            return UnlockError::Failed(format!("Fingerprint unlock failed: {e}"));
        };
        let message = resp.message.clone().unwrap_or_default();
        match resp.code.as_deref() {
            Some("userCancel") | Some("systemCancel") => UnlockError::Cancelled,
            Some("biometryLockout") => UnlockError::Failed(
                "Too many attempts. Unlock with your master password.".into(),
            ),
            Some("biometryNotEnrolled") | Some("biometryNotAvailable") => UnlockError::Failed(
                "No fingerprint is set up on this phone.".into(),
            ),
            // The Keystore key was invalidated (a fingerprint was added or the
            // screen lock removed) or the data no longer matches it.
            _ if message.starts_with("Failed to initialize the cipher")
                || message.starts_with("Failed to decrypt") =>
            {
                UnlockError::Invalidated(
                    "Fingerprint unlock was turned off because the phone's fingerprints or screen lock changed. Unlock with your master password, then turn it on again in Settings.".into(),
                )
            }
            _ => UnlockError::Failed(format!("Fingerprint unlock failed: {message}")),
        }
    }
}

// ── Windows: Windows Hello key credential ────────────────────────────────────

#[cfg(windows)]
mod backend {
    use super::{wipe, UnlockError, B64};
    use base64::Engine;
    use chacha20poly1305::aead::{Aead, KeyInit, Payload};
    use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
    use rand::RngCore;
    use sha2::{Digest, Sha256};
    use windows::core::{Array, HSTRING};
    use windows::Security::Credentials::{
        KeyCredentialCreationOption, KeyCredentialManager, KeyCredentialStatus,
    };
    use windows::Security::Cryptography::CryptographicBuffer;
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

    /// Name of SecurePass's Windows Hello key credential (per Windows user).
    const CREDENTIAL_NAME: &str = "SecurePass vault unlock";
    const AAD: &[u8] = b"securepass-windows-hello-v1";
    const CHALLENGE_LEN: usize = 32;
    const NONCE_LEN: usize = 12;

    /// WinRT needs the calling thread initialised (these run on blocking-pool
    /// threads); "already initialised" is fine.
    fn init_winrt() {
        let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
    }

    pub fn is_available(_app: &tauri::AppHandle) -> bool {
        init_winrt();
        KeyCredentialManager::IsSupportedAsync()
            .and_then(|op| op.join())
            .unwrap_or(false)
    }

    fn status_error(status: KeyCredentialStatus) -> UnlockError {
        match status {
            KeyCredentialStatus::UserCanceled | KeyCredentialStatus::UserPrefersPassword => UnlockError::Cancelled,
            KeyCredentialStatus::NotFound => UnlockError::Invalidated(
                "Windows Hello unlock was turned off because its Windows Hello key was removed. Unlock with your master password, then turn it on again in Settings.".into(),
            ),
            KeyCredentialStatus::SecurityDeviceLocked => UnlockError::Failed(
                "Windows Hello is locked after too many attempts. Unlock with your master password.".into(),
            ),
            _ => UnlockError::Failed(format!("Windows Hello failed (status {}).", status.0)),
        }
    }

    fn win_err(e: windows::core::Error) -> UnlockError {
        UnlockError::Failed(format!("Windows Hello failed: {}", e.message()))
    }

    /// Ask Windows Hello to verify the user and sign `challenge` with
    /// SecurePass's key credential (creating a new one first when `create`).
    fn hello_sign(challenge: &[u8], create: bool) -> Result<Vec<u8>, UnlockError> {
        init_winrt();
        bring_hello_dialog_to_front();
        let name = HSTRING::from(CREDENTIAL_NAME);
        let retrieval = if create {
            KeyCredentialManager::RequestCreateAsync(&name, KeyCredentialCreationOption::ReplaceExisting)
        } else {
            KeyCredentialManager::OpenAsync(&name)
        }
        .and_then(|op| op.join())
        .map_err(win_err)?;
        let status = retrieval.Status().map_err(win_err)?;
        if status != KeyCredentialStatus::Success {
            return Err(status_error(status));
        }
        let credential = retrieval.Credential().map_err(win_err)?;

        let buffer = CryptographicBuffer::CreateFromByteArray(challenge).map_err(win_err)?;
        bring_hello_dialog_to_front();
        let signed = credential
            .RequestSignAsync(&buffer)
            .and_then(|op| op.join())
            .map_err(win_err)?;
        let status = signed.Status().map_err(win_err)?;
        if status != KeyCredentialStatus::Success {
            return Err(status_error(status));
        }
        let mut signature = Array::<u8>::new();
        CryptographicBuffer::CopyToByteArray(&signed.Result().map_err(win_err)?, &mut signature).map_err(win_err)?;
        Ok(signature.to_vec())
    }

    /// The vault key is wrapped with SHA-256 of Hello's (deterministic RSA)
    /// signature over the stored challenge.
    fn wrapping_cipher(mut signature: Vec<u8>) -> ChaCha20Poly1305 {
        let digest = Sha256::digest(&signature);
        wipe(&mut signature);
        ChaCha20Poly1305::new(Key::from_slice(digest.as_slice()))
    }

    pub fn protect(_app: &tauri::AppHandle, key: &[u8; 32]) -> Result<String, String> {
        let mut challenge = [0u8; CHALLENGE_LEN];
        rand::thread_rng().fill_bytes(&mut challenge);
        let signature = hello_sign(&challenge, true).map_err(|e| match e {
            UnlockError::Cancelled => "Windows Hello setup was cancelled.".to_string(),
            UnlockError::Invalidated(m) | UnlockError::Failed(m) => m,
            UnlockError::PasswordRequired => unreachable!(),
        })?;
        let cipher = wrapping_cipher(signature);
        let mut nonce = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce);
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), Payload { msg: key, aad: AAD })
            .map_err(|_| "Couldn't protect the vault key.".to_string())?;
        Ok(B64.encode([&challenge[..], &nonce[..], &ciphertext[..]].concat()))
    }

    pub fn unprotect(_app: &tauri::AppHandle, data: &str) -> Result<[u8; 32], UnlockError> {
        let unreadable = || UnlockError::Invalidated("The stored Windows Hello key was unreadable.".into());
        let raw = B64.decode(data.as_bytes()).map_err(|_| unreadable())?;
        if raw.len() <= CHALLENGE_LEN + NONCE_LEN {
            return Err(unreadable());
        }
        let (challenge, rest) = raw.split_at(CHALLENGE_LEN);
        let (nonce, ciphertext) = rest.split_at(NONCE_LEN);

        let cipher = wrapping_cipher(hello_sign(challenge, false)?);
        let mut plain = cipher
            .decrypt(Nonce::from_slice(nonce), Payload { msg: ciphertext, aad: AAD })
            // A different signature: the Hello credential was replaced.
            .map_err(|_| UnlockError::Invalidated(
                "Windows Hello unlock was turned off because its Windows Hello key changed. Unlock with your master password, then turn it on again in Settings.".into(),
            ))?;
        let key = <[u8; 32]>::try_from(plain.as_slice()).map_err(|_| unreadable());
        wipe(&mut plain);
        key
    }

    /// The Hello dialog of a desktop (Win32) app can open behind its window.
    /// Watch briefly for it and bring it to the front (best effort).
    fn bring_hello_dialog_to_front() {
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow};
        std::thread::spawn(|| {
            for _ in 0..40 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if let Ok(hwnd) = unsafe { FindWindowW(w!("Credential Dialog Xaml Host"), None) } {
                    if !hwnd.is_invalid() {
                        let _ = unsafe { SetForegroundWindow(hwnd) };
                        return;
                    }
                }
            }
        });
    }
}

// ── Everything else: not available ───────────────────────────────────────────

#[cfg(not(any(target_os = "android", windows)))]
mod backend {
    use super::UnlockError;

    pub fn is_available(_app: &tauri::AppHandle) -> bool {
        false
    }
    pub fn protect(_app: &tauri::AppHandle, _key: &[u8; 32]) -> Result<String, String> {
        Err("Biometric unlock isn't available on this platform.".into())
    }
    pub fn unprotect(_app: &tauri::AppHandle, _data: &str) -> Result<[u8; 32], UnlockError> {
        Err(UnlockError::Failed("Biometric unlock isn't available on this platform.".into()))
    }
}
