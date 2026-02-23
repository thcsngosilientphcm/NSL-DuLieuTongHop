// src-tauri/src/commands/misc.rs
// Commands phụ trợ

use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn focus_main_window(app: AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_focus();
    }
}
