#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri::plugin_updater::Builder::new().build()) // Kích hoạt Update
        .plugin(tauri::plugin_process::init()) // Kích hoạt Process (để restart app)
        .plugin(tauri::plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}