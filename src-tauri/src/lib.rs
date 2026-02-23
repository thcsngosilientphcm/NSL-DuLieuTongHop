// src-tauri/src/lib.rs
// Registry: đăng ký modules, plugins và commands

mod commands;
mod core;
mod menus;

use commands::browser_cmds;
use commands::misc;
use menus::quanlymatkhau;

// --- ENTRY ---
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            // Account commands
            quanlymatkhau::get_all_accounts,
            quanlymatkhau::save_account,
            quanlymatkhau::delete_account,
            quanlymatkhau::get_full_account_details,
            quanlymatkhau::refresh_autofill_data,
            // Browser commands
            browser_cmds::open_embedded_browser,
            browser_cmds::update_embedded_browser_bounds,
            browser_cmds::hide_embedded_browser,
            // Misc commands
            misc::focus_main_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
