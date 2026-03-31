use tauri::command;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;


use crate::crypto::CryptoManager;
use crate::vault::{Vault, Entry, FieldType, VaultMetadata};

/// App state containing the unlocked vault and crypto manager
pub struct AppState {
    pub vault: Option<Vault>,
    pub crypto: Option<CryptoManager>,
    pub is_locked: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            vault: None,
            crypto: None,
            is_locked: true,
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

    let decrypted_bytes = crypto.decrypt(ciphertext, &nonce, b"vault").map_err(|e| {
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

#[command]
pub async fn import_vault(
    state: tauri::State<'_, Mutex<AppState>>,
    request: ImportVaultRequest,
) -> Result<VaultMetadata, String> {
    let mut state = state.lock().map_err(|e| format!("State lock error: {}", e))?;

    // Load vault from the specified file path
    let vault = load_vault_from_path(&request.file_path, &request.master_password)?;

    // Create crypto manager for the imported vault
    let crypto = CryptoManager::new(&request.master_password, &vault.salt)
        .map_err(|e| format!("Failed to create crypto manager: {}", e))?;

    // Update state
    state.vault = Some(vault);
    state.crypto = Some(crypto);
    state.is_locked = false;

    let metadata = VaultMetadata::from(state.vault.as_ref().unwrap());
    Ok(metadata)
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