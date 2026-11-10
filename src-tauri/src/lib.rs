// SecurePass - Encrypted Password Manager
// Modules
mod crypto;
mod vault;
mod commands;
mod sync;

use std::sync::Mutex;
use commands::AppState;
use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

/// Shrinks the window to fit the monitor's work area (the screen minus the
/// taskbar/dock, wherever it is docked) and centres it there. Tauri's own
/// `center` uses the full monitor, which puts the title bar under a top
/// taskbar, and a fixed 800 px height can overflow small or scaled screens.
fn fit_to_work_area(window: &WebviewWindow) -> tauri::Result<()> {
    let monitor = match window.current_monitor()? {
        Some(m) => m,
        None => match window.primary_monitor()? {
            Some(m) => m,
            None => return Ok(()),
        },
    };
    let area = *monitor.work_area();
    let outer = window.outer_size()?;
    let inner = window.inner_size()?;

    // Title bar + borders, so the whole outer frame fits, not just the content.
    let deco_w = outer.width.saturating_sub(inner.width);
    let deco_h = outer.height.saturating_sub(inner.height);

    let width = outer.width.min(area.size.width);
    let height = outer.height.min(area.size.height);
    if width != outer.width || height != outer.height {
        window.set_size(PhysicalSize::new(
            width.saturating_sub(deco_w),
            height.saturating_sub(deco_h),
        ))?;
    }

    let x = area.position.x + ((area.size.width - width) / 2) as i32;
    let y = area.position.y + ((area.size.height - height) / 2) as i32;
    window.set_position(PhysicalPosition::new(x, y))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(AppState::default()))
        .setup(|app| {
            // The window is created hidden (tauri.conf.json) so it appears
            // already in place instead of jumping after it is moved.
            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = fit_to_work_area(&window) {
                    eprintln!("Could not fit window to the work area: {e}");
                }
                window.show()?;
                window.set_focus()?;
            }
            Ok(())
        })
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
            commands::import_from_password_locker,
            commands::get_sync_status,
            commands::connect_google_drive,
            commands::disconnect_google_drive,
            commands::sync_vault,
            commands::resolve_conflicts,
            commands::check_sync_needed,
            commands::restore_vault_from_drive,
            commands::reauth_and_sync,
            commands::change_master_password,
            commands::check_for_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running SecurePass application");
}
