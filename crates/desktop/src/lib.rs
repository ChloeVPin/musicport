//! desktop shell (tauri) around core

mod commands;

use commands::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            // bring window to front on launch
            if let Some(win) = app.get_webview_window("main") {
                win.show().ok();
                win.unminimize().ok();
                win.set_focus().ok();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connect,
            commands::list_devices,
            commands::list_tracks,
            commands::add_files,
            commands::remove_tracks,
            commands::export_tracks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the musicport application");
}