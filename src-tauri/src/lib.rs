// SecurePass - Encrypted Password Manager
// Modules
mod crypto;
mod vault;
mod commands;

use std::sync::Mutex;
use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(AppState::default()))
        .invoke_handler(tauri::generate_handler![
            commands::create_vault,
            commands::unlock_vault,
            commands::lock_vault,
            commands::get_vault_status,
            commands::create_entry,
            commands::update_entry,
            commands::delete_entry,
            commands::get_entries,
            commands::get_entry,
            commands::search_entries,
            commands::get_categories,
            commands::get_tags,
            commands::copy_to_clipboard,
            commands::clear_clipboard,
            commands::generate_password,
            commands::export_vault,
            commands::import_vault,
            commands::import_from_password_locker
        ])
        .run(tauri::generate_context!())
        .expect("error while running SecurePass application");
}
