// src-tauri/src/lib.rs
mod database;
mod scripts;

use std::time::Duration;
// use std::sync::Mutex;

use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde_json::Value;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Rect, WebviewBuilder, WebviewUrl,
};

use crate::database::{
    get_all_accounts_impl,
    load_store,
    perform_save_account,
    resolve_data_path,
    save_store,
    url_decode,
    AccountDTO,
    SECRET_KEY, // [FIX] Đã bỏ AccountStatus, check_account_status
};
use crate::scripts::get_autofill_script;

// --- COMMANDS DATABASE ---
#[tauri::command]
fn get_all_accounts(app: AppHandle) -> Vec<AccountDTO> {
    get_all_accounts_impl(&app)
}
#[tauri::command]
fn save_account(
    app: AppHandle,
    domain: String,
    user: String,
    pass: String,
) -> Result<String, String> {
    perform_save_account(&app, domain, user, pass)
}
#[tauri::command]
fn delete_account(app: AppHandle, domain: String, username: String) -> Result<String, String> {
    let clean = domain
        .replace("https://", "")
        .replace("http://", "")
        .split('/')
        .next()
        .unwrap_or(&domain)
        .to_string();
    let mut store = load_store(&app);
    if let Some(list) = store.accounts.get_mut(&clean) {
        list.retain(|a| a.user != username);
        save_store(&app, &store)?;
        return Ok("OK".to_string());
    }
    Err("ERR".to_string())
}
#[tauri::command]
fn get_full_account_details(
    app: AppHandle,
    domain: String,
    username: String,
) -> Result<(String, String), String> {
    let clean = domain
        .replace("https://", "")
        .replace("http://", "")
        .split('/')
        .next()
        .unwrap_or(&domain)
        .to_string();
    let store = load_store(&app);
    let mc = new_magic_crypt!(SECRET_KEY, 256);
    if let Some(list) = store.accounts.get(&clean) {
        if let Some(acc) = list.iter().find(|a| a.user == username) {
            let pass = mc.decrypt_base64_to_string(&acc.pass).unwrap_or_default();
            return Ok((acc.user.clone(), pass));
        }
    }
    Err("Not found".to_string())
}
#[tauri::command]
fn reveal_data_file(app: AppHandle) {
    let path = resolve_data_path(&app);
    if let Some(parent) = path.parent() {
        let _ = std::process::Command::new("explorer").arg(parent).spawn();
    }
}
#[tauri::command]
fn nuke_data_file(app: AppHandle) {
    let path = resolve_data_path(&app);
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
}
#[tauri::command]
fn refresh_autofill_data(app: AppHandle, url: String) {
    if url.is_empty() {
        return;
    }
    if let Some(view) = app.get_webview("browser_view") {
        let accounts_json = get_accounts_json_for_domain(&app, &url);
        let js = format!("if(typeof window.__NSL_UPDATE_ACCOUNTS__ === 'function') {{ window.__NSL_UPDATE_ACCOUNTS__({}); }}", accounts_json);
        let _ = view.eval(&js);
    }
}
#[tauri::command]
fn focus_main_window(app: AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_focus();
    }
}

// --- LOGIC MỚI: EMBEDDED BROWSER (FIXED API V2) ---

#[tauri::command]
async fn open_embedded_browser(
    app: AppHandle,
    url: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let accounts_json = get_accounts_json_for_domain(&app, &url);
    let autofill_script = get_autofill_script(&accounts_json);
    let safe_url = url.replace('\\', "\\\\").replace('\'', "\\'");

    // 1. Kiểm tra xem Webview con đã tồn tại chưa
    if let Some(view) = app.get_webview("browser_view") {
        // Cập nhật vị trí
        let _ = view.set_bounds(Rect {
            position: PhysicalPosition { x, y }.into(),
            size: PhysicalSize { width, height }.into(),
        });
        let _ = view.show();
        let _ = view.set_focus();

        // Inject data mới
        let js_update = format!("if(typeof window.__NSL_UPDATE_ACCOUNTS__ === 'function') {{ window.__NSL_UPDATE_ACCOUNTS__({}); }}", accounts_json);
        let _ = view.eval(&js_update);

        // Navigate nếu URL khác
        if let Ok(current) = view.url() {
            if current.to_string() != url {
                let _ = view.eval(&format!("window.location.href = '{}';", safe_url));
            }
        } else {
            let _ = view.eval(&format!("window.location.href = '{}';", safe_url));
        }
    } else {
        // 2. Tạo mới Webview con (Gắn vào window Main)
        println!("[NSL] Creating Embedded Browser View...");
        let main_win = app
            .get_webview_window("main")
            .ok_or("Main window not found")?;

        // Dùng Window::add_child() thay vì WebviewBuilder::build() (API mới Tauri 2.10+)
        let builder =
            WebviewBuilder::new("browser_view", WebviewUrl::External(url.parse().unwrap()))
                .initialization_script(&autofill_script);

        let webview = main_win
            .as_ref()
            .window()
            .add_child(
                builder,
                PhysicalPosition { x, y },
                PhysicalSize { width, height },
            )
            .map_err(|e| format!("Failed to create webview: {}", e))?;

        // Setup logic lắng nghe login
        let app_clone = app.clone();
        setup_browser_monitor(webview, app_clone);
    }
    Ok(())
}

#[tauri::command]
fn update_embedded_browser_bounds(app: AppHandle, x: i32, y: i32, width: u32, height: u32) {
    if let Some(view) = app.get_webview("browser_view") {
        let _ = view.set_bounds(Rect {
            position: PhysicalPosition { x, y }.into(),
            size: PhysicalSize { width, height }.into(),
        });
    }
}

#[tauri::command]
fn hide_embedded_browser(app: AppHandle) {
    if let Some(view) = app.get_webview("browser_view") {
        let _ = view.hide();
    }
}

// Stub functions để JS không bị lỗi
#[tauri::command]
fn update_webview_layout(_app: AppHandle, _args: Value) {}
#[tauri::command]
fn hide_embedded_view(_app: AppHandle) {}
#[tauri::command]
async fn navigate_webview(_app: AppHandle, _url: String) {}
#[tauri::command]
async fn open_secure_window(_app: AppHandle, _args: Value) -> Result<(), String> {
    Ok(())
}

// --- HELPERS ---
fn get_accounts_json_for_domain(app: &AppHandle, raw_url: &str) -> String {
    let clean_domain = raw_url
        .replace("https://", "")
        .replace("http://", "")
        .split('/')
        .next()
        .unwrap_or("")
        .to_string();
    let store = load_store(app);
    let mc = new_magic_crypt!(SECRET_KEY, 256);
    let mut accounts_vec = Vec::new();
    if let Some(list) = store.accounts.get(&clean_domain) {
        for acc in list {
            let pass_plain = mc.decrypt_base64_to_string(&acc.pass).unwrap_or_default();
            accounts_vec.push(AccountDTO {
                id: acc.user.clone(),
                domain: clean_domain.clone(),
                website: raw_url.to_string(),
                username: acc.user.clone(),
                password: pass_plain,
            });
        }
    }
    serde_json::to_string(&accounts_vec).unwrap_or_else(|_| "[]".to_string())
}

fn setup_browser_monitor(view: tauri::Webview, app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_processed_hash = String::new();
        loop {
            // Kiểm tra webview còn sống không
            if let Err(_) = view.url() {
                break;
            }

            if let Ok(current_url) = view.url() {
                if let Some(fragment) = current_url.fragment() {
                    if fragment != last_processed_hash {
                        last_processed_hash = fragment.to_string();
                        // Xử lý lệnh SAVE
                        if fragment.starts_with("NSL_CMD_SAVE|") {
                            let parts: Vec<&str> = fragment.split('|').collect();
                            if parts.len() >= 3 {
                                let json_str = url_decode(parts[2]);
                                if let Ok(val) = serde_json::from_str::<Value>(&json_str) {
                                    let u = val["user"].as_str().unwrap_or("").to_string();
                                    let p = val["pass"].as_str().unwrap_or("").to_string();
                                    let _ =
                                        perform_save_account(&app, current_url.to_string(), u, p);
                                    let _ = app.emit("refresh-accounts", ());
                                }
                            }
                            let _ = view.eval("history.replaceState(null, null, ' ');");
                        } else if fragment.starts_with("NSL_TRIGGER|") {
                            // Logic xử lý khi user bấm đăng nhập (nếu cần)
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

// --- ENTRY ---
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_all_accounts,
            save_account,
            delete_account,
            get_full_account_details,
            refresh_autofill_data,
            focus_main_window,
            open_embedded_browser,
            update_embedded_browser_bounds,
            hide_embedded_browser,
            update_webview_layout,
            hide_embedded_view,
            navigate_webview,
            open_secure_window,
            reveal_data_file,
            nuke_data_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
