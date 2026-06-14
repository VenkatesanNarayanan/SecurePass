use tauri::command;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;
use chrono::Utc;

use crate::crypto::CryptoManager;
use crate::vault::{Vault, Entry, FieldType, VaultMetadata};
use crate::sync;

/// App state containing the unlocked vault and crypto manager
pub struct AppState {
    pub vault: Option<Vault>,
    pub crypto: Option<CryptoManager>,
    pub is_locked: bool,
    /// Partially-merged vault waiting for conflict resolution.
    pub pending_sync_merge: Option<Vault>,
    /// Conflicted entry pairs (local, remote) to be shown to the user.
    pub pending_conflicts: Vec<(Entry, Entry)>,
    /// Drive file ID kept here during conflict resolution so we can finalize the upload.
    pub pending_drive_file_id: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            vault: None,
            crypto: None,
            is_locked: true,
            pending_sync_merge: None,
            pending_conflicts: Vec::new(),
            pending_drive_file_id: None,
        }
    }
}

/// Request/Response types for Tauri commands

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVaultRequest {
    pub master_password: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockVaultRequest {
    pub master_password: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEntryRequest {
    pub category: String,
    pub title: String,
    pub fields: Vec<FieldRequest>,
    pub tags: Vec<String>,
    pub notes: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldRequest {
    pub name: String,
    pub field_type: String, // "text", "password", "long_text"
    pub value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEntryRequest {
    pub entry_id: String,
    pub category: Option<String>,
    pub title: Option<String>,
    pub fields: Option<Vec<FieldRequest>>,
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
    pub category_filter: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetEntryRequest {
    pub entry_id: String,
}

/// Get the vault file path
fn get_vault_path() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".securepass")
        .join("vault.dat")
}

/// Save vault to encrypted file
fn save_vault(vault: &Vault, crypto: &CryptoManager) -> Result<(), String> {
    let json = serde_json::to_string(vault).map_err(|e| e.to_string())?;
    let plaintext = json.as_bytes();
    let nonce = CryptoManager::generate_nonce();
    let aad = b"vault";
    let ciphertext = crypto.encrypt(plaintext, &nonce, aad).map_err(|e| e.to_string())?;

    let mut data = vault.salt.to_vec();
    data.extend(nonce.to_vec());
    data.extend(ciphertext);

    let path = get_vault_path();
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())?;
    Ok(())
}

/// Load vault from encrypted file
fn load_vault(master_password: &str) -> Result<Vault, String> {
    let path = get_vault_path();
    if !path.exists() {
        return Err("Vault not found".to_string());
    }

    let data = fs::read(&path).map_err(|e| e.to_string())?;
    if data.len() < 44 {
        return Err("Invalid vault file".to_string());
    }

    let salt_bytes: [u8; 32] = data[0..32].try_into().map_err(|_| "Invalid salt".to_string())?;
    let nonce_bytes: [u8; 12] = data[32..44].try_into().map_err(|_| "Invalid nonce".to_string())?;
    let nonce = chacha20poly1305::Nonce::from(nonce_bytes);
    let ciphertext = &data[44..];

    let crypto = CryptoManager::new(master_password, &salt_bytes).map_err(|e| e.to_string())?;

    let decrypted_bytes = crypto.decrypt(ciphertext, &nonce, b"vault").map_err(|_e| {
        // Return a user-friendly error message for wrong password
        "Incorrect master password".to_string()
    })?;

    let json = String::from_utf8(decrypted_bytes).map_err(|e| e.to_string())?;
    let vault: Vault = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(vault)
}

/// Convert string field type to enum
fn parse_field_type(field_type: &str) -> Result<FieldType, String> {
    match field_type {
        "text" => Ok(FieldType::Text),
        "password" => Ok(FieldType::Password),
        "long_text" => Ok(FieldType::LongText),
        _ => Err(format!("Invalid field type: {}", field_type)),
    }
}

/// Tauri Commands

#[command]
pub async fn create_vault(
    state: tauri::State<'_, Mutex<AppState>>,
    request: CreateVaultRequest,
) -> Result<VaultMetadata, String> {
    let mut state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    // Create new vault
    let mut vault = Vault::new();

    // Create crypto manager with the master password
    let crypto = CryptoManager::new(&request.master_password, &vault.salt)
        .map_err(|e| format!("Failed to create crypto manager: {:?}", e))?;

    // Create a default entry for testing
    let mut test_entry = Entry::new("web-app".to_string(), "Welcome to SecurePass".to_string());
    test_entry.notes = "This is your first entry. Feel free to edit or delete it.".to_string();

    // Add the test entry to vault
    vault.add_entry(test_entry);

    // Save vault to file
    save_vault(&vault, &crypto).map_err(|e| format!("Failed to save vault: {}", e))?;

    // Update state
    state.vault = Some(vault);
    state.crypto = Some(crypto);
    state.is_locked = false;

    let metadata = VaultMetadata::from(state.vault.as_ref().unwrap());
    Ok(metadata)
}

#[command]
pub async fn unlock_vault(
    state: tauri::State<'_, Mutex<AppState>>,
    request: UnlockVaultRequest,
) -> Result<VaultMetadata, String> {
    let mut state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    if state.vault.is_none() {
        // Try to load vault from file
        let vault = load_vault(&request.master_password)?;
        state.vault = Some(vault);
    }

    // Get the salt before creating crypto manager
    let salt = state.vault.as_ref().unwrap().salt;

    // Create crypto manager
    let crypto = CryptoManager::new(&request.master_password, &salt)
        .map_err(|e| format!("Failed to unlock vault: {}", e))?;

    state.crypto = Some(crypto);
    state.is_locked = false;

    let metadata = VaultMetadata::from(state.vault.as_ref().unwrap());
    Ok(metadata)
}

#[command]
pub async fn lock_vault(state: tauri::State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    // Zeroize crypto key for security
    if let Some(ref mut crypto) = state.crypto {
        crypto.zeroize_key();
    }

    // Clear vault from memory for security - forces re-validation on unlock
    state.vault = None;
    state.crypto = None;
    state.is_locked = true;

    Ok(())
}

#[command]
pub async fn get_vault_status(state: tauri::State<'_, Mutex<AppState>>) -> Result<serde_json::Value, String> {
    let state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    // Check if vault file exists on disk
    let vault_file_exists = get_vault_path().exists();

    let status = serde_json::json!({
        "is_locked": state.is_locked,
        "has_vault": vault_file_exists,
        "entry_count": state.vault.as_ref().map(|v| v.entries.len()).unwrap_or(0),
    });

    Ok(status)
}

#[command]
pub async fn create_entry(
    state: tauri::State<'_, Mutex<AppState>>,
    request: CreateEntryRequest,
) -> Result<String, String> {
    let mut state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    if state.is_locked || state.vault.is_none() {
        return Err("Vault is locked".to_string());
    }

    // Create new entry
    let mut entry = Entry::new(request.category, request.title);
    entry.tags = request.tags;
    entry.notes = request.notes;

    // Add fields (temporary implementation - no encryption yet)
    for field_req in request.fields {
        let field_type = parse_field_type(&field_req.field_type)
            .map_err(|e| format!("Invalid field type: {}", e))?;
        entry.add_field(field_req.name, field_type, field_req.value);
    }

    let entry_id = entry.id.to_string();

    {
        let vault = state.vault.as_mut().unwrap();
        vault.add_entry(entry);
    }

    // Save vault after adding entry
    {
        let crypto = state.crypto.as_ref().unwrap();
        let vault = state.vault.as_ref().unwrap();
        save_vault(vault, crypto).map_err(|e| format!("Failed to save vault: {}", e))?;
    }

    Ok(entry_id)
}

#[command]
pub async fn get_entries(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Vec<serde_json::Value>, String> {
    let state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    if state.is_locked || state.vault.is_none() {
        return Err("Vault is locked".to_string());
    }

    let vault = state.vault.as_ref().unwrap();

    // Convert entries to JSON (without decrypted field values for security)
    let entries: Vec<serde_json::Value> = vault.entries.iter().map(|entry| {
        serde_json::json!({
            "id": entry.id,
            "category": entry.category,
            "title": entry.title,
            "tags": entry.tags,
            "notes": entry.notes,
            "created_at": entry.created_at,
            "updated_at": entry.updated_at,
            "field_count": entry.fields.len(),
        })
    }).collect();

    Ok(entries)
}

#[command]
pub async fn get_entry(
    state: tauri::State<'_, Mutex<AppState>>,
    request: GetEntryRequest,
) -> Result<serde_json::Value, String> {
    let state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    if state.is_locked || state.vault.is_none() {
        return Err("Vault is locked".to_string());
    }

    let vault = state.vault.as_ref().unwrap();
    let crypto = state.crypto.as_ref().unwrap();

    let entry_uuid = Uuid::parse_str(&request.entry_id)
        .map_err(|e| format!("Invalid entry ID: {}", e))?;

    let entry = vault.get_entry(&entry_uuid)
        .ok_or_else(|| "Entry not found".to_string())?;

    // Get field values
    let mut fields = Vec::new();
    for field in &entry.fields {
        let value = field.decrypt_value(crypto)
            .or_else(|_| field.get_plaintext_value())
            .map_err(|e| format!("Failed to get field value: {}", e))?;

        let field_type_str = match field.field_type {
            crate::vault::FieldType::Text => "text",
            crate::vault::FieldType::Password => "password",
            crate::vault::FieldType::LongText => "long_text",
        };

        fields.push(serde_json::json!({
            "name": field.name,
            "fieldType": field_type_str,
            "value": value,
        }));
    }

    let entry_json = serde_json::json!({
        "id": entry.id,
        "category": entry.category,
        "title": entry.title,
        "fields": fields,
        "tags": entry.tags,
        "notes": entry.notes,
        "created_at": entry.created_at,
        "updated_at": entry.updated_at,
    });

    Ok(entry_json)
}

#[command]
pub async fn search_entries(
    state: tauri::State<'_, Mutex<AppState>>,
    request: SearchRequest,
) -> Result<Vec<serde_json::Value>, String> {
    let state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    if state.is_locked || state.vault.is_none() {
        return Err("Vault is locked".to_string());
    }

    let vault = state.vault.as_ref().unwrap();

    // Perform search
    let mut results = if request.query.is_empty() {
        vault.entries.iter().collect::<Vec<_>>()
    } else {
        vault.search(&request.query)
    };

    // Apply category filter
    if let Some(category) = request.category_filter {
        results = results.into_iter()
            .filter(|entry| entry.category == category)
            .collect();
    }

    // Convert to JSON
    let entries: Vec<serde_json::Value> = results.into_iter().map(|entry| {
        serde_json::json!({
            "id": entry.id,
            "category": entry.category,
            "title": entry.title,
            "tags": entry.tags,
            "notes": entry.notes,
            "created_at": entry.created_at,
            "updated_at": entry.updated_at,
            "field_count": entry.fields.len(),
        })
    }).collect();

    Ok(entries)
}

#[command]
pub async fn get_categories(state: tauri::State<'_, Mutex<AppState>>) -> Result<Vec<String>, String> {
    let state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    if state.is_locked || state.vault.is_none() {
        return Err("Vault is locked".to_string());
    }

    let vault = state.vault.as_ref().unwrap();
    Ok(vault.get_categories())
}

#[command]
pub async fn update_entry(
    state: tauri::State<'_, Mutex<AppState>>,
    request: UpdateEntryRequest,
) -> Result<(), String> {
    let mut state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    if state.is_locked || state.vault.is_none() {
        return Err("Vault is locked".to_string());
    }

    let entry_uuid = Uuid::parse_str(&request.entry_id)
        .map_err(|e| format!("Invalid entry ID: {}", e))?;

    {
        let vault = state.vault.as_mut().unwrap();
        let entry = vault.get_entry_mut(&entry_uuid)
            .ok_or_else(|| "Entry not found".to_string())?;

        // Update basic fields
        if let Some(category) = request.category {
            entry.category = category;
        }
        if let Some(title) = request.title {
            entry.title = title;
        }
        if let Some(notes) = request.notes {
            entry.notes = notes;
        }
        if let Some(tags) = request.tags {
            entry.tags = tags;
        }

        // Update fields if provided
        if let Some(fields) = request.fields {
            entry.fields.clear();
            for field_req in fields {
                let field_type = parse_field_type(&field_req.field_type)
                    .map_err(|e| format!("Invalid field type: {}", e))?;
                entry.add_field(field_req.name.clone(), field_type, field_req.value.clone());
            }
        }
    }

    // Save vault after updating entry
    {
        let crypto = state.crypto.as_ref().unwrap();
        let vault = state.vault.as_ref().unwrap();
        save_vault(vault, crypto).map_err(|e| format!("Failed to save vault: {}", e))?;
    }

    Ok(())
}

#[command]
pub async fn delete_entry(
    state: tauri::State<'_, Mutex<AppState>>,
    request: GetEntryRequest,
) -> Result<(), String> {
    let mut state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    if state.is_locked || state.vault.is_none() {
        return Err("Vault is locked".to_string());
    }

    let entry_uuid = Uuid::parse_str(&request.entry_id)
        .map_err(|e| format!("Invalid entry ID: {}", e))?;

    {
        let vault = state.vault.as_mut().unwrap();
        if !vault.remove_entry(&entry_uuid) {
            return Err("Entry not found".to_string());
        }
    }

    // Save vault after deleting entry
    {
        let crypto = state.crypto.as_ref().unwrap();
        let vault = state.vault.as_ref().unwrap();
        save_vault(vault, crypto).map_err(|e| format!("Failed to save vault: {}", e))?;
    }

    Ok(())
}

#[command]
pub async fn get_tags(state: tauri::State<'_, Mutex<AppState>>) -> Result<Vec<String>, String> {
    let state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    if state.is_locked || state.vault.is_none() {
        return Err("Vault is locked".to_string());
    }

    let vault = state.vault.as_ref().unwrap();
    Ok(vault.get_tags())
}

#[command]
pub async fn copy_to_clipboard(text: String) -> Result<(), String> {
    use std::process::Command;

    // Use system clipboard commands
    #[cfg(target_os = "macos")]
    {
        let mut child = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn pbcopy: {}", e))?;

        use std::io::Write;
        child.stdin.as_mut().unwrap().write_all(text.as_bytes())
            .map_err(|e| format!("Failed to write to clipboard: {}", e))?;

        child.wait().map_err(|e| format!("Failed to wait for pbcopy: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        let mut child = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn xclip: {}", e))?;

        use std::io::Write;
        child.stdin.as_mut().unwrap().write_all(text.as_bytes())
            .map_err(|e| format!("Failed to write to clipboard: {}", e))?;

        child.wait().map_err(|e| format!("Failed to wait for xclip: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        let mut child = Command::new("clip")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn clip: {}", e))?;

        use std::io::Write;
        child.stdin.as_mut().unwrap().write_all(text.as_bytes())
            .map_err(|e| format!("Failed to write to clipboard: {}", e))?;

        child.wait().map_err(|e| format!("Failed to wait for clip: {}", e))?;
    }

    Ok(())
}

#[command]
pub async fn clear_clipboard() -> Result<(), String> {
    // Copy empty string to clear clipboard
    copy_to_clipboard("".to_string()).await
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratePasswordRequest {
    pub length: usize,
    pub charset: String,
}

#[command]
pub async fn generate_password(request: GeneratePasswordRequest) -> Result<String, String> {
    use rand::Rng;

    if request.charset.is_empty() {
        return Err("Charset cannot be empty".to_string());
    }

    if request.length == 0 {
        return Err("Password length must be greater than 0".to_string());
    }

    let charset_bytes = request.charset.as_bytes();
    let charset_len = charset_bytes.len();

    let mut rng = rand::thread_rng();
    let password: String = (0..request.length)
        .map(|_| {
            let idx = rng.gen_range(0..charset_len);
            charset_bytes[idx] as char
        })
        .collect();

    Ok(password)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportVaultRequest {
    pub file_path: String,
}

#[command]
pub async fn export_vault(
    state: tauri::State<'_, Mutex<AppState>>,
    request: ExportVaultRequest,
) -> Result<(), String> {
    let state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    if state.is_locked || state.vault.is_none() || state.crypto.is_none() {
        return Err("Vault is locked".to_string());
    }

    let vault = state.vault.as_ref().unwrap();
    let crypto = state.crypto.as_ref().unwrap();

    // Save vault to the specified file path
    save_vault_to_path(vault, crypto, &request.file_path)?;

    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportVaultRequest {
    pub file_path: String,
    pub master_password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportVaultResult {
    pub entries_added: usize,
    pub entries_kept_local: usize,
}

#[command]
pub async fn import_vault(
    state: tauri::State<'_, Mutex<AppState>>,
    request: ImportVaultRequest,
) -> Result<ImportVaultResult, String> {
    let mut state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    if state.is_locked || state.vault.is_none() || state.crypto.is_none() {
        return Err("Vault is locked".to_string());
    }

    // Decrypt the imported vault file
    let imported = load_vault_from_path(&request.file_path, &request.master_password)?;

    let local = state.vault.as_ref().unwrap();
    let crypto = state.crypto.as_ref().unwrap();

    // Merge: add entries from the imported vault that don't exist locally.
    // last_sync = None means epoch, so every entry is "changed" — conflicts
    // (same UUID, different updated_at) are resolved by keeping the local version.
    let merge = sync::merge_vaults(local, &imported, None);

    let conflicts_count = merge.raw_conflicts.len();
    let entries_added = merge.entries_added;

    let mut merged = merge.merged;
    // Re-add the local side of every conflict (merge_vaults drops them from merged).
    for (local_entry, _) in merge.raw_conflicts {
        merged.entries.push(local_entry);
    }
    merged.updated_at = chrono::Utc::now();

    // Persist merged vault to disk
    save_vault(&merged, crypto).map_err(|e| format!("Failed to save vault: {}", e))?;

    state.vault = Some(merged);

    Ok(ImportVaultResult {
        entries_added,
        entries_kept_local: conflicts_count,
    })
}

/// Save vault to a specific file path
fn save_vault_to_path(vault: &Vault, crypto: &CryptoManager, file_path: &str) -> Result<(), String> {
    let json = serde_json::to_string(vault).map_err(|e| e.to_string())?;
    let plaintext = json.as_bytes();
    let nonce = CryptoManager::generate_nonce();
    let aad = b"vault";
    let ciphertext = crypto.encrypt(plaintext, &nonce, aad).map_err(|e| e.to_string())?;

    let mut data = vault.salt.to_vec();
    data.extend(nonce.to_vec());
    data.extend(ciphertext);

    std::fs::write(file_path, data).map_err(|e| e.to_string())?;
    Ok(())
}

// ---- PasswordLocker JSON import ----

#[derive(Deserialize)]
struct PasswordLockerField {
    key: String,
    value: String,
    value_type: String,
}

#[derive(Deserialize)]
struct PasswordLockerEntry {
    title: String,
    icon: Option<String>,
    fields: Vec<PasswordLockerField>,
}

fn icon_to_category(icon: &str) -> &'static str {
    if icon.contains('🌐') || icon.contains('🌍') || icon.contains('🌎') || icon.contains('🔗') {
        "web-app"
    } else if icon.contains('🖥') || icon.contains('💻') || icon.contains('🖧') {
        "ssh-server"
    } else if icon.contains('🏦') || icon.contains('💳') || icon.contains('💰') || icon.contains('🏧') {
        "bank-financial"
    } else if icon.contains('📧') || icon.contains('✉') || icon.contains('📨') || icon.contains('📩') {
        "email"
    } else if icon.contains('📶') || icon.contains('📡') || icon.contains('🛜') {
        "wi-fi"
    } else {
        "generic"
    }
}

fn value_type_to_field_type(value_type: &str) -> FieldType {
    match value_type.to_lowercase().as_str() {
        "password" | "pw" => FieldType::Password,
        "long_text" | "longtext" | "textarea" | "notes" => FieldType::LongText,
        _ => FieldType::Text,
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFromPasswordLockerRequest {
    pub file_path: String,
}

#[command]
pub async fn import_from_password_locker(
    state: tauri::State<'_, Mutex<AppState>>,
    request: ImportFromPasswordLockerRequest,
) -> Result<usize, String> {
    let mut state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    if state.is_locked || state.vault.is_none() || state.crypto.is_none() {
        return Err("Vault is locked".to_string());
    }

    let json = std::fs::read_to_string(&request.file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let pl_entries: Vec<PasswordLockerEntry> = serde_json::from_str(&json)
        .map_err(|e| format!("Invalid PasswordLocker export file: {}", e))?;

    let count = pl_entries.len();

    {
        let vault = state.vault.as_mut().unwrap();
        for pl_entry in pl_entries {
            let category = pl_entry.icon.as_deref()
                .map(icon_to_category)
                .unwrap_or("generic")
                .to_string();
            let mut entry = Entry::new(category, pl_entry.title);
            for field in pl_entry.fields {
                let field_type = value_type_to_field_type(&field.value_type);
                entry.add_field(field.key, field_type, field.value);
            }
            vault.add_entry(entry);
        }
    }

    {
        let crypto = state.crypto.as_ref().unwrap();
        let vault = state.vault.as_ref().unwrap();
        save_vault(vault, crypto).map_err(|e| format!("Failed to save vault: {}", e))?;
    }

    Ok(count)
}

// ── Cloud storage / sync commands ────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusResponse {
    pub connected: bool,
    pub last_sync: Option<chrono::DateTime<Utc>>,
    pub has_pending_conflicts: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConflictInfo {
    pub local_entry: serde_json::Value,
    pub remote_entry: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub status: String,
    pub conflicts: Vec<ConflictInfo>,
    pub entries_added: usize,
    pub entries_updated: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictResolution {
    pub entry_id: String,
    pub keep: String, // "local" or "remote"
}

fn entry_to_json(entry: &Entry) -> serde_json::Value {
    let fields: Vec<serde_json::Value> = entry.fields.iter().map(|f| {
        let value = String::from_utf8(f.value_encrypted.clone()).unwrap_or_default();
        let type_str = match f.field_type {
            FieldType::Password => "password",
            FieldType::LongText => "long_text",
            FieldType::Text => "text",
        };
        serde_json::json!({ "name": f.name, "fieldType": type_str, "value": value })
    }).collect();

    serde_json::json!({
        "id": entry.id,
        "title": entry.title,
        "category": entry.category,
        "notes": entry.notes,
        "tags": entry.tags,
        "fields": fields,
        "updatedAt": entry.updated_at,
    })
}

/// Return the current Google Drive connection status.
#[command]
pub async fn get_sync_status(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<SyncStatusResponse, String> {
    let has_pending = {
        let s = state.lock().map_err(|e| e.to_string())?;
        !s.pending_conflicts.is_empty()
    };
    let sync_state = sync::load_sync_state();
    Ok(SyncStatusResponse {
        connected: sync_state.connected,
        last_sync: sync_state.last_sync,
        has_pending_conflicts: has_pending,
    })
}

/// Start the Google Drive OAuth flow. Opens the user's browser; blocks until
/// they complete sign-in (timeout 5 min).
#[command]
pub async fn connect_google_drive(
    _state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    sync::start_oauth_flow().await?;
    let mut sync_state = sync::load_sync_state();
    sync_state.connected = true;
    sync::save_sync_state(&sync_state)?;
    Ok(())
}

/// Remove stored tokens and disconnect from Google Drive.
#[command]
pub async fn disconnect_google_drive(
    _state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    sync::clear_cloud_data()
}

/// Sync the local vault with Google Drive.
///
/// Returns `status = "synced"` when everything merged automatically, or
/// `status = "conflicts"` with a list of conflicting entries for the user
/// to resolve via `resolve_conflicts`.
#[command]
pub async fn sync_vault(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<SyncResult, String> {
    // Extract what we need from locked state, then release the lock before
    // making async network calls.
    let (key_bytes, local_vault, sync_state) = {
        let s = state.lock().map_err(|e| e.to_string())?;
        if s.is_locked || s.vault.is_none() {
            return Err("Vault is locked".to_string());
        }
        let sync_state = sync::load_sync_state();
        if !sync_state.connected {
            return Err("Not connected to Google Drive. Connect in Settings first.".to_string());
        }
        (
            s.crypto.as_ref().unwrap().get_key_bytes(),
            s.vault.as_ref().unwrap().clone(),
            sync_state,
        )
    };

    let crypto = CryptoManager::from_key_bytes(&key_bytes);
    let access_token = sync::get_valid_access_token().await?;
    let file_id = sync::find_drive_file(&access_token).await?;

    // ── First sync: nothing on Drive yet ────────────────────────────────────
    if file_id.is_none() {
        let vault_bytes = sync::encrypt_vault_bytes(&local_vault, &crypto)?;
        let new_id = sync::upload_vault_to_drive(&access_token, vault_bytes, None).await?;

        let mut new_state = sync_state;
        new_state.drive_file_id = Some(new_id);
        new_state.last_sync = Some(Utc::now());
        sync::save_sync_state(&new_state)?;

        return Ok(SyncResult {
            status: "synced".to_string(),
            conflicts: vec![],
            entries_added: local_vault.entries.len(),
            entries_updated: 0,
        });
    }

    // ── Download + decrypt remote vault ─────────────────────────────────────
    let file_id = file_id.unwrap();
    let remote_bytes = sync::download_vault_from_drive(&access_token, &file_id).await?;
    let remote_vault = match sync::decrypt_vault_bytes(&remote_bytes, &crypto) {
        Ok(v) => v,
        Err(_) => {
            // Decryption failed with the current key — the master password was
            // changed on another device. Signal the frontend to ask for the new password.
            return Ok(SyncResult {
                status: "password_changed".to_string(),
                conflicts: vec![],
                entries_added: 0,
                entries_updated: 0,
            });
        }
    };

    // ── Merge ────────────────────────────────────────────────────────────────
    let merge = sync::merge_vaults(&local_vault, &remote_vault, sync_state.last_sync);

    if merge.raw_conflicts.is_empty() {
        // No conflicts: finalize immediately.
        // Only upload to Drive when local actually contributed changes — skipping the upload
        // when local had nothing new prevents bumping Drive's modifiedTime and avoids
        // triggering spurious "sync needed" indicators on other devices.
        if merge.local_contributed {
            let vault_bytes = sync::encrypt_vault_bytes(&merge.merged, &crypto)?;
            sync::upload_vault_to_drive(&access_token, vault_bytes, Some(&file_id)).await?;
        }

        let (added, updated) = (merge.entries_added, merge.entries_updated);
        let merged_vault = merge.merged;

        {
            let mut s = state.lock().map_err(|e| e.to_string())?;
            save_vault(&merged_vault, s.crypto.as_ref().unwrap())
                .map_err(|e| format!("Failed to save merged vault: {}", e))?;
            s.vault = Some(merged_vault);
        }

        let mut new_state = sync_state;
        new_state.drive_file_id = Some(file_id);
        new_state.last_sync = Some(Utc::now());
        sync::save_sync_state(&new_state)?;

        Ok(SyncResult { status: "synced".to_string(), conflicts: vec![], entries_added: added, entries_updated: updated })
    } else {
        // Has conflicts: stash pending state and return conflict info to the frontend.
        let conflicts: Vec<ConflictInfo> = merge.raw_conflicts.iter()
            .map(|(l, r)| ConflictInfo {
                local_entry: entry_to_json(l),
                remote_entry: entry_to_json(r),
            })
            .collect();

        {
            let mut s = state.lock().map_err(|e| e.to_string())?;
            s.pending_sync_merge = Some(merge.merged);
            s.pending_conflicts = merge.raw_conflicts;
            s.pending_drive_file_id = Some(file_id);
        }

        Ok(SyncResult {
            status: "conflicts".to_string(),
            conflicts,
            entries_added: merge.entries_added,
            entries_updated: merge.entries_updated,
        })
    }
}

/// Called after `sync_vault` returns `status = "password_changed"`.
///
/// System B is still unlocked with the old key (vault already in memory).
/// This command downloads the Drive vault, reads the new salt embedded in its
/// header, derives the new key from the user-supplied new password + that salt,
/// decrypts and merges Drive entries with local entries, re-saves locally using
/// the new key, and updates AppState — so both devices share the same key.
#[command]
pub async fn reauth_and_sync(
    state: tauri::State<'_, Mutex<AppState>>,
    new_master_password: String,
) -> Result<SyncResult, String> {
    if new_master_password.len() < 8 {
        return Err("New master password must be at least 8 characters".to_string());
    }

    // Grab local vault + sync state under the lock, then release before async ops.
    let (local_vault, sync_state) = {
        let s = state.lock().map_err(|e| e.to_string())?;
        if s.is_locked || s.vault.is_none() {
            return Err("Vault is locked".to_string());
        }
        let ss = sync::load_sync_state();
        if !ss.connected {
            return Err("Not connected to Google Drive".to_string());
        }
        (s.vault.as_ref().unwrap().clone(), ss)
    };

    let access_token = sync::get_valid_access_token().await?;
    let file_id = sync::find_drive_file(&access_token)
        .await?
        .ok_or("No SecurePass vault found on Google Drive")?;

    // Download the Drive vault (encrypted with the new password + new salt from System A).
    let remote_bytes = sync::download_vault_from_drive(&access_token, &file_id).await?;
    if remote_bytes.len() < 44 {
        return Err("Drive vault appears corrupted (file too short)".to_string());
    }

    // The new salt is the first 32 bytes of the Drive file — we MUST use this
    // salt (not generate a new one) so key derivation matches System A.
    let new_salt: [u8; 32] = remote_bytes[0..32]
        .try_into()
        .map_err(|_| "Could not read salt from Drive vault")?;

    let new_crypto = CryptoManager::new(&new_master_password, &new_salt)
        .map_err(|e| format!("Failed to derive new key: {}", e))?;

    // Decryption validates the password — wrong password fails here.
    let remote_vault = sync::decrypt_vault_bytes(&remote_bytes, &new_crypto)
        .map_err(|_| "Incorrect new master password".to_string())?;

    // Merge: local in-memory entries + remote entries.
    let merge = sync::merge_vaults(&local_vault, &remote_vault, sync_state.last_sync);
    let mut merged = merge.merged;
    // Merged vault must carry the new salt so save_vault writes the right header.
    merged.salt = new_salt;
    merged.updated_at = Utc::now();

    if merge.raw_conflicts.is_empty() {
        save_vault(&merged, &new_crypto)
            .map_err(|e| format!("Failed to save vault: {}", e))?;

        let vault_bytes = sync::encrypt_vault_bytes(&merged, &new_crypto)?;
        sync::upload_vault_to_drive(&access_token, vault_bytes, Some(&file_id)).await?;

        let (added, updated) = (merge.entries_added, merge.entries_updated);
        let new_key_bytes = new_crypto.get_key_bytes();

        {
            let mut s = state.lock().map_err(|e| e.to_string())?;
            s.vault = Some(merged);
            // Replace crypto so future saves use the new key.
            s.crypto = Some(CryptoManager::from_key_bytes(&new_key_bytes));
        }

        let mut new_state = sync_state;
        new_state.drive_file_id = Some(file_id);
        new_state.last_sync = Some(Utc::now());
        sync::save_sync_state(&new_state)?;

        Ok(SyncResult {
            status: "synced".to_string(),
            conflicts: vec![],
            entries_added: added,
            entries_updated: updated,
        })
    } else {
        // Stash conflicts for the user to resolve via resolve_conflicts.
        let conflicts: Vec<ConflictInfo> = merge.raw_conflicts.iter()
            .map(|(l, r)| ConflictInfo {
                local_entry: entry_to_json(l),
                remote_entry: entry_to_json(r),
            })
            .collect();

        let new_key_bytes = new_crypto.get_key_bytes();

        {
            let mut s = state.lock().map_err(|e| e.to_string())?;
            // Switch crypto now so resolve_conflicts uses the new key when it saves.
            s.crypto = Some(CryptoManager::from_key_bytes(&new_key_bytes));
            // Update vault salt so any interim save uses the right header.
            if let Some(v) = s.vault.as_mut() { v.salt = new_salt; }
            s.pending_sync_merge = Some(merged);
            s.pending_conflicts = merge.raw_conflicts;
            s.pending_drive_file_id = Some(file_id);
        }

        Ok(SyncResult {
            status: "conflicts".to_string(),
            conflicts,
            entries_added: merge.entries_added,
            entries_updated: merge.entries_updated,
        })
    }
}

/// Change the vault's master password.
///
/// Verifies the current password, generates a fresh Argon2 salt, re-derives
/// the key from the new password, re-encrypts the vault, saves locally, and
/// (if Google Drive is connected) re-uploads so other devices stay in sync.
#[command]
pub async fn change_master_password(
    state: tauri::State<'_, Mutex<AppState>>,
    current_password: String,
    new_password: String,
) -> Result<(), String> {
    if new_password.len() < 8 {
        return Err("New password must be at least 8 characters".to_string());
    }

    // Phase 1: verify current password, re-encrypt, save locally — all under the lock.
    let (new_key_bytes, vault_snapshot, sync_connected) = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        if s.is_locked || s.vault.is_none() {
            return Err("Vault is locked".to_string());
        }

        let current_key = s.crypto.as_ref().unwrap().get_key_bytes();
        let vault = s.vault.as_ref().unwrap();

        // Verify by deriving from the stored salt and comparing key bytes.
        let check = CryptoManager::new(&current_password, &vault.salt)
            .map_err(|_| "Failed to verify current password".to_string())?;
        if check.get_key_bytes() != current_key {
            return Err("Current password is incorrect".to_string());
        }

        // New salt → new key (salt reuse would weaken security).
        let new_salt = CryptoManager::generate_salt();
        let new_crypto = CryptoManager::new(&new_password, &new_salt)
            .map_err(|e| format!("Failed to create new encryption key: {}", e))?;

        let vault = s.vault.as_mut().unwrap();
        vault.salt = new_salt;
        vault.updated_at = Utc::now();

        save_vault(vault, &new_crypto)
            .map_err(|e| format!("Failed to save vault: {}", e))?;

        let new_key_bytes = new_crypto.get_key_bytes();
        let snapshot = vault.clone();
        let connected = sync::load_sync_state().connected;

        s.crypto = Some(new_crypto);

        (new_key_bytes, snapshot, connected)
    };

    // Phase 2: re-upload to Drive outside the lock (async).
    if sync_connected {
        let crypto = CryptoManager::from_key_bytes(&new_key_bytes);
        if let Ok(token) = sync::get_valid_access_token().await {
            if let Ok(file_id) = sync::find_drive_file(&token).await {
                if let Ok(bytes) = sync::encrypt_vault_bytes(&vault_snapshot, &crypto) {
                    if let Ok(new_id) = sync::upload_vault_to_drive(&token, bytes, file_id.as_deref()).await {
                        let mut ss = sync::load_sync_state();
                        ss.drive_file_id = Some(new_id);
                        ss.last_sync = Some(Utc::now());
                        let _ = sync::save_sync_state(&ss);
                    }
                }
            }
        }
        // Non-fatal — local vault is already saved with the new key.
    }

    Ok(())
}

/// Check whether a sync is needed without downloading the full vault.
///
/// Compares the local vault's `updated_at` and the Drive file's `modifiedTime`
/// against `last_sync`. Returns immediately (one lightweight Drive metadata call).
#[command]
pub async fn check_sync_needed(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<serde_json::Value, String> {
    let (vault_updated_at, sync_state) = {
        let s = state.lock().map_err(|e| e.to_string())?;
        if s.is_locked || s.vault.is_none() {
            return Err("Vault is locked".to_string());
        }
        let ss = sync::load_sync_state();
        if !ss.connected {
            return Ok(serde_json::json!({ "needsSync": false, "localChanged": false, "remoteChanged": false }));
        }
        (s.vault.as_ref().unwrap().updated_at, ss)
    };

    let last_sync = sync_state.last_sync.unwrap_or(chrono::DateTime::<Utc>::MIN_UTC);
    let local_changed = vault_updated_at > last_sync;

    // No file ID yet → first-time sync is always needed.
    let Some(file_id) = sync_state.drive_file_id else {
        return Ok(serde_json::json!({ "needsSync": true, "localChanged": true, "remoteChanged": false }));
    };

    let access_token = sync::get_valid_access_token().await?;
    let remote_modified = sync::get_drive_file_modified_time(&access_token, &file_id).await?;
    let remote_changed = remote_modified.map(|t| t > last_sync).unwrap_or(false);

    Ok(serde_json::json!({
        "needsSync": local_changed || remote_changed,
        "localChanged": local_changed,
        "remoteChanged": remote_changed,
    }))
}

/// Apply the user's conflict resolutions and complete the sync.
///
/// Each resolution specifies an `entryId` and `keep: "local" | "remote"`.
#[command]
pub async fn resolve_conflicts(
    state: tauri::State<'_, Mutex<AppState>>,
    resolutions: Vec<ConflictResolution>,
) -> Result<(), String> {
    let (key_bytes, pending_merge, pending_conflicts, drive_file_id) = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        let merge = s.pending_sync_merge.take()
            .ok_or("No pending sync to resolve")?;
        let conflicts = std::mem::take(&mut s.pending_conflicts);
        let fid = s.pending_drive_file_id.take()
            .ok_or("No pending Drive file ID")?;
        let key = s.crypto.as_ref().unwrap().get_key_bytes();
        (key, merge, conflicts, fid)
    };

    let mut merged = pending_merge;

    // Apply each resolution by adding the chosen entry version to the merged vault.
    for res in &resolutions {
        let entry_uuid = Uuid::parse_str(&res.entry_id)
            .map_err(|e| format!("Invalid entry ID: {}", e))?;

        if let Some((local_e, remote_e)) = pending_conflicts.iter().find(|(l, _)| l.id == entry_uuid) {
            let chosen = if res.keep == "local" { local_e } else { remote_e };
            merged.entries.push(chosen.clone());
        }
    }
    merged.updated_at = Utc::now();

    let crypto = CryptoManager::from_key_bytes(&key_bytes);
    let vault_bytes = sync::encrypt_vault_bytes(&merged, &crypto)?;
    let access_token = sync::get_valid_access_token().await?;
    sync::upload_vault_to_drive(&access_token, vault_bytes, Some(&drive_file_id)).await?;

    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        save_vault(&merged, s.crypto.as_ref().unwrap())
            .map_err(|e| format!("Failed to save vault: {}", e))?;
        s.vault = Some(merged);
    }

    let mut sync_state = sync::load_sync_state();
    sync_state.drive_file_id = Some(drive_file_id);
    sync_state.last_sync = Some(Utc::now());
    sync::save_sync_state(&sync_state)?;

    Ok(())
}

/// Restore a vault from Google Drive onto a device that has no local vault.
///
/// This is the Device-B first-launch path. It MUST be called after
/// `connect_google_drive` (tokens already stored). It downloads the encrypted
/// vault from Drive, extracts the salt embedded in the file, derives the
/// decryption key from the user's master password + that salt, decrypts, saves
/// locally, and marks the sync state as up-to-date.
#[command]
pub async fn restore_vault_from_drive(
    state: tauri::State<'_, Mutex<AppState>>,
    master_password: String,
) -> Result<VaultMetadata, String> {
    // Tokens must have been stored by connect_google_drive already.
    let access_token = sync::get_valid_access_token().await?;

    let file_id = sync::find_drive_file(&access_token)
        .await?
        .ok_or("No SecurePass vault found on Google Drive. Make sure you have synced at least once from your other device, then try again.")?;

    let vault_bytes = sync::download_vault_from_drive(&access_token, &file_id).await?;

    if vault_bytes.len() < 44 {
        return Err("Vault file on Google Drive is too short — it may be corrupted.".to_string());
    }

    // The first 32 bytes of the file ARE the Argon2 salt used when the vault
    // was originally created. We must use this salt (not generate a new one)
    // so that key derivation produces the same key as the original device.
    let salt: [u8; 32] = vault_bytes[0..32]
        .try_into()
        .map_err(|_| "Could not read salt from vault file")?;

    let crypto = CryptoManager::new(&master_password, &salt)
        .map_err(|e| format!("Failed to initialise encryption: {}", e))?;

    // A wrong password will produce a wrong key and fail authentication here.
    let vault = sync::decrypt_vault_bytes(&vault_bytes, &crypto)
        .map_err(|_| "Incorrect master password or corrupted vault file.".to_string())?;

    save_vault(&vault, &crypto)
        .map_err(|e| format!("Failed to save vault locally: {}", e))?;

    let mut sync_state = sync::load_sync_state();
    sync_state.connected = true;
    sync_state.drive_file_id = Some(file_id);
    sync_state.last_sync = Some(Utc::now());
    sync::save_sync_state(&sync_state)?;

    let metadata = VaultMetadata::from(&vault);
    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.vault = Some(vault);
        s.crypto = Some(crypto);
        s.is_locked = false;
    }

    Ok(metadata)
}

// ── Update check ─────────────────────────────────────────────────────────────

#[command]
pub async fn check_for_update() -> Result<serde_json::Value, String> {
    let current = env!("CARGO_PKG_VERSION");

    let resp = reqwest::Client::new()
        .get("https://www.securepass.wonderstek.com/version.json")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;

    let latest = resp["version"].as_str().unwrap_or("").to_string();
    let has_update = is_newer(&latest, current);

    Ok(serde_json::json!({
        "hasUpdate": has_update,
        "currentVersion": current,
        "latestVersion": latest,
        "releasedAt": resp["released_at"],
        "downloadUrl": resp["download_url"],
    }))
}

/// Returns true if `remote` is a strictly higher semver than `local`.
fn is_newer(remote: &str, local: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let mut parts = v.trim_start_matches('v').splitn(3, '.');
        let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
        let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
        let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    };
    parse(remote) > parse(local)
}

// ── Vault file helpers (used by non-sync commands above) ─────────────────────

/// Load vault from a specific file path
fn load_vault_from_path(file_path: &str, master_password: &str) -> Result<Vault, String> {
    let data = std::fs::read(file_path).map_err(|e| e.to_string())?;
    if data.len() < 44 {
        return Err("Invalid vault file".to_string());
    }

    let salt_bytes: [u8; 32] = data[0..32].try_into().map_err(|_| "Invalid salt".to_string())?;
    let nonce_bytes: [u8; 12] = data[32..44].try_into().map_err(|_| "Invalid nonce".to_string())?;
    let nonce = chacha20poly1305::Nonce::from(nonce_bytes);
    let ciphertext = &data[44..];

    let crypto = CryptoManager::new(master_password, &salt_bytes).map_err(|e| e.to_string())?;
    let decrypted_bytes = crypto.decrypt(ciphertext, &nonce, b"vault").map_err(|e| e.to_string())?;
    let json = String::from_utf8(decrypted_bytes).map_err(|e| e.to_string())?;
    let vault: Vault = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(vault)
}