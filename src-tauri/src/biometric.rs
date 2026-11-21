//! Fingerprint unlock (Android only).
//!
//! The 32-byte vault key — never the master password — is encrypted with an
//! AES key that lives in the Android Keystore and can only be used after a
//! strong biometric (fingerprint) authentication, via `tauri-plugin-biometric`'s
//! `biometric_cipher` (BiometricPrompt + CryptoObject). The resulting
//! ciphertext is stored in `biometric_unlock.json` in the app's data dir.
//! The decrypted key stays in Rust; it is never sent to the web UI.
//!
//! The enrolment is dropped when it can no longer be trusted or used:
//! - Android invalidates the Keystore key when a fingerprint is added, so
//!   decryption fails and the enrolment is deleted;
//! - the vault's salt or key changes (master password changed here or on
//!   another device, vault restored or re-created): the stored key no longer
//!   opens the vault file, and those commands also call [`clear`].
//!
//! Even with a valid enrolment, the master password is required again once
//! [`PASSWORD_INTERVAL_SECS`] have passed since the last password unlock.
//!
//! On every other platform the functions report "not available" and do nothing.

/// The master password must be typed at least this often (7 days).
pub const PASSWORD_INTERVAL_SECS: i64 = 7 * 24 * 60 * 60;

/// Why a fingerprint unlock didn't produce a key.
#[derive(Debug, PartialEq)]
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
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

#[cfg(target_os = "android")]
pub use android::*;

#[cfg(not(target_os = "android"))]
pub use other::*;

#[cfg(target_os = "android")]
mod android {
    use super::{UnlockError, PASSWORD_INTERVAL_SECS};
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use tauri::Manager;
    use tauri_plugin_biometric::{AuthOptions, BiometricExt};

    const FILE_NAME: &str = "biometric_unlock.json";

    #[derive(Serialize, Deserialize)]
    struct Enrollment {
        version: u32,
        /// Keystore-encrypted vault key, as returned by the plugin ("data;iv").
        data: String,
        /// Salt of the vault the key belongs to (base64), to spot a re-keyed vault.
        vault_salt: String,
        /// Unix time of the last unlock with the master password.
        last_password_unlock: i64,
    }

    fn path(app: &tauri::AppHandle) -> PathBuf {
        app.path().app_data_dir().expect("app data dir").join(FILE_NAME)
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

    /// Overwrite a string's bytes before it is dropped (best effort: the
    /// plugin's JVM side keeps its own immutable copy until GC).
    fn wipe(s: String) {
        let mut bytes = s.into_bytes();
        bytes.iter_mut().for_each(|b| *b = 0);
        std::hint::black_box(&bytes);
    }

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

    /// Strong biometrics enrolled and usable on this device.
    pub fn is_available(app: &tauri::AppHandle) -> bool {
        app.biometric().status().map(|s| s.is_available).unwrap_or(false)
    }

    pub fn is_enrolled(app: &tauri::AppHandle) -> bool {
        read(app).is_some()
    }

    /// Forget the enrolment (the Keystore key itself is reused next time).
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

    /// Prompt for a fingerprint and store `key`, encrypted with the
    /// biometric-bound Keystore key.
    pub fn enroll(app: &tauri::AppHandle, key: &[u8; 32], vault_salt: &[u8; 32]) -> Result<(), String> {
        let mut opts = options("Turn on fingerprint unlock", "Confirm with your fingerprint");
        opts.data_to_encrypt = Some(B64.encode(key));
        let result = app
            .biometric()
            .biometric_cipher("Turn on fingerprint unlock for SecurePass".into(), opts)
            .map_err(|e| match classify(&e) {
                UnlockError::Cancelled => "Fingerprint setup was cancelled.".to_string(),
                UnlockError::Invalidated(m) | UnlockError::Failed(m) => m,
                UnlockError::PasswordRequired => unreachable!(),
            })?;
        write(
            app,
            &Enrollment {
                version: 1,
                data: result.data,
                vault_salt: B64.encode(vault_salt),
                last_password_unlock: now(),
            },
        )
    }

    /// Prompt for a fingerprint and return the vault key for the vault whose
    /// file currently has `vault_salt`.
    pub fn unlock_key(app: &tauri::AppHandle, vault_salt: &[u8; 32]) -> Result<[u8; 32], UnlockError> {
        let enrollment = read(app).ok_or_else(|| {
            UnlockError::Invalidated("Fingerprint unlock isn't set up.".into())
        })?;

        if enrollment.vault_salt != B64.encode(vault_salt) {
            clear(app);
            return Err(UnlockError::Invalidated(
                "The vault was re-encrypted (new master password or restore). Unlock with your master password, then turn fingerprint unlock on again in Settings.".into(),
            ));
        }
        if now() - enrollment.last_password_unlock > PASSWORD_INTERVAL_SECS {
            return Err(UnlockError::PasswordRequired);
        }

        let mut opts = options("Unlock SecurePass", "Use your fingerprint to open your vault");
        opts.data_to_decrypt = Some(enrollment.data);
        let result = app
            .biometric()
            .biometric_cipher("Unlock SecurePass".into(), opts)
            .map_err(|e| {
                let err = classify(&e);
                if matches!(err, UnlockError::Invalidated(_)) {
                    clear(app);
                }
                err
            })?;

        let decoded = B64.decode(result.data.as_bytes());
        wipe(result.data);
        let mut bytes = decoded.map_err(|_| {
            clear(app);
            UnlockError::Invalidated("The stored fingerprint key was unreadable.".into())
        })?;
        let key: Result<[u8; 32], _> = bytes.as_slice().try_into();
        bytes.iter_mut().for_each(|b| *b = 0);
        key.map_err(|_| {
            clear(app);
            UnlockError::Invalidated("The stored fingerprint key was unreadable.".into())
        })
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

#[cfg(not(target_os = "android"))]
mod other {
    use super::UnlockError;

    pub fn is_available(_app: &tauri::AppHandle) -> bool {
        false
    }
    pub fn is_enrolled(_app: &tauri::AppHandle) -> bool {
        false
    }
    pub fn clear(_app: &tauri::AppHandle) {}
    pub fn note_password_unlock(_app: &tauri::AppHandle) {}
    pub fn enroll(_app: &tauri::AppHandle, _key: &[u8; 32], _salt: &[u8; 32]) -> Result<(), String> {
        Err("Fingerprint unlock is only available on Android.".into())
    }
    pub fn unlock_key(_app: &tauri::AppHandle, _salt: &[u8; 32]) -> Result<[u8; 32], UnlockError> {
        Err(UnlockError::Failed("Fingerprint unlock is only available on Android.".into()))
    }
}
