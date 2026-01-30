use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, LogicalPosition, LogicalSize, Url};
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde::{Deserialize, Serialize};
use base64::{engine::general_purpose, Engine as _};

// --- CẤU TRÚC DỮ LIỆU ---
#[derive(Serialize, Deserialize, Debug, Clone)]
struct AccountStore {
    accounts: HashMap<String, (String, String)>,
}

#[derive(Serialize)]
struct AccountDTO {
    domain: String,
    username: String,
}

const SECRET_KEY: &str = "NSL_SECURE_KEY_2026_HCM"; 

// --- CÁC HÀM HELPER (QUAN TRỌNG: PHẢI NẰM Ở ĐÂY) ---

fn get_creds_path(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap().join("creds.json")
}

fn load_store(app: &AppHandle) -> AccountStore {
    let path = get_creds_path(app);
    if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or(AccountStore { accounts: HashMap::new() })
    } else {
        AccountStore { accounts: HashMap::new() }
    }
}

fn save_store(app: &AppHandle, store: &AccountStore) -> Result<(), String> {
    let path = get_creds_path(app);
    if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

// Hàm này bị thiếu ở lần trước -> Gây ra lỗi
fn perform_save_account(app: &AppHandle, domain: String, user: String, pass: String) -> Result<String, String> {
    let mut store = load_store(app);
    let mc = new_magic_crypt!(SECRET_KEY, 256);
    
    // Kiểm tra trùng lặp (nếu user & pass y hệt thì không lưu đè để đỡ tốn I/O)
    if let Some((stored_user, stored_pass_enc)) = store.accounts.get(&domain) {
        if stored_user == &user {
            if let Ok(stored_pass_dec) = mc.decrypt_base64_to_string(stored_pass_enc) {
                if stored_pass_dec == pass {
                    return Ok("Dữ liệu không đổi".to_string());
                }
            }
        }
    }

    let encrypted_pass = mc.encrypt_str_to_base64(&pass);
    store.accounts.insert(domain, (user, encrypted_pass));
    save_store(app, &store)?;
    Ok("Đã lưu thành công!".to_string())
}

// --- CÁC COMMAND GỌI TỪ JAVASCRIPT ---

#[tauri::command]
fn get_all_accounts(app: AppHandle) -> Vec<AccountDTO> {
    let store = load_store(&app);
    let mut list: Vec<AccountDTO> = store.accounts.iter().map(|(k, v)| {
        AccountDTO { domain: k.clone(), username: v.0.clone() }
    }).collect();
    list.sort_by(|a, b| a.domain.cmp(&b.domain));
    list
}

#[tauri::command]
fn get_password_plaintext(app: AppHandle, domain: String) -> Result<String, String> {
    let store = load_store(&app);
    if let Some((_, p_enc)) = store.accounts.get(&domain) {
        let mc = new_magic_crypt!(SECRET_KEY, 256);
        if let Ok(p_dec) = mc.decrypt_base64_to_string(p_enc) {
            return Ok(p_dec);
        }
    }
    Err("Không tìm thấy".to_string())
}

#[tauri::command]
fn delete_account(app: AppHandle, domain: String) -> Result<String, String> {
    let mut store = load_store(&app);
    if store.accounts.remove(&domain).is_some() {
        save_store(&app, &store)?;
        Ok("Đã xóa".to_string())
    } else {
        Err("Không tồn tại".to_string())
    }
}

#[tauri::command]
fn save_account(app: AppHandle, domain: String, user: String, pass: String) -> Result<String, String> {
    perform_save_account(&app, domain, user, pass)
}

#[tauri::command]
fn update_webview_layout(app: AppHandle, sidebar_width: f64) {
    if let Some(win) = app.get_webview_window("embedded_browser") {
        if let Some(main_window) = app.get_webview_window("main") {
            let size = main_window.inner_size().unwrap();
            let header_height = 64.0;
            
            let _ = win.set_position(LogicalPosition::new(sidebar_width, header_height));
            let _ = win.set_size(LogicalSize::new(
                (size.width as f64) - sidebar_width, 
                (size.height as f64) - header_height
            ));
        }
    }
}

#[tauri::command]
fn hide_embedded_view(app: AppHandle) {
    if let Some(win) = app.get_webview_window("embedded_browser") {
        let _ = win.close(); 
    }
}

#[tauri::command]
async fn navigate_webview(app: AppHandle, url: String) {
    if let Some(win) = app.get_webview_window("embedded_browser") {
        let script = format!("window.location.replace('{}')", url);
        let _ = win.eval(&script);
    }
}

#[tauri::command]
async fn open_secure_window(app: AppHandle, url: String) {
    let domain_raw = url.replace("https://", "").replace("http://", "");
    let domain = domain_raw.split('/').next().unwrap_or("").to_string();
    
    // Lấy pass đã lưu để auto-fill
    let store = load_store(&app);
    let mut username = String::new();
    let mut password = String::new();

    if let Some((u, p_enc)) = store.accounts.get(&domain) {
        let mc = new_magic_crypt!(SECRET_KEY, 256);
        if let Ok(p_dec) = mc.decrypt_base64_to_string(p_enc) {
            username = u.clone();
            password = p_dec;
        }
    }

    let init_script = format!(r#"
        window.addEventListener('DOMContentLoaded', () => {{
            function autoClickTab() {{
                let spans = document.querySelectorAll('.rtsTxt');
                for (let span of spans) {{
                    if (span.innerText.trim() === "Tài khoản QLTH") {{
                        let link = span.closest('a.rtsLink');
                        if (link) link.click(); return;
                    }}
                }}
            }}
            function autoFill() {{
                const u = "{}"; const p = "{}";
                if(u) {{
                    let ui = document.querySelector('input[name*="UserName"]') || document.querySelector('input[id*="user"]');
                    let pi = document.querySelector('input[name*="Password"]') || document.querySelector('input[id*="pass"]');
                    if(ui && pi && !ui.value) {{
                        ui.value = u; pi.value = p;
                        ui.dispatchEvent(new Event('input', {{bubbles:true}}));
                        pi.dispatchEvent(new Event('input', {{bubbles:true}}));
                        let cap = document.querySelector('input[name*="Captcha"]');
                        if(cap) cap.focus();
                    }}
                }}
            }}
            // Logic bắt mật khẩu mới
            function setupCapture() {{
                function sendToRust() {{
                    let ui = document.querySelector('input[name*="UserName"]') || document.querySelector('input[id*="user"]');
                    let pi = document.querySelector('input[name*="Password"]') || document.querySelector('input[id*="pass"]');
                    if (ui && pi && ui.value && pi.value) {{
                        let u64 = btoa(unescape(encodeURIComponent(ui.value)));
                        let p64 = btoa(unescape(encodeURIComponent(pi.value)));
                        window.location.replace("https://nsl.local/save/" + u64 + "/" + p64);
                    }}
                }}
                document.addEventListener('keydown', (e) => {{ if (e.key === 'Enter') sendToRust(); }});
                document.addEventListener('click', (e) => {{
                    let t = e.target;
                    while (t && t !== document) {{
                        if (t.type === 'submit' || t.innerText.toLowerCase().includes('đăng nhập')) {{ sendToRust(); break; }}
                        t = t.parentElement;
                    }}
                }});
            }}
            setTimeout(autoClickTab, 500);
            setTimeout(autoFill, 800);
            setupCapture();
        }});
    "#, username, password);

    if let Some(win) = app.get_webview_window("embedded_browser") { let _ = win.close(); }
    
    let main_window = app.get_webview_window("main").unwrap();
    let size = main_window.inner_size().unwrap();
    
    // Mặc định sidebar mở (260px)
    let webview_x = 260.0;
    let webview_y = 64.0;
    let webview_w = (size.width as f64) - webview_x;
    let webview_h = (size.height as f64) - webview_y;
    
    let app_handle_clone = app.clone();
    
    let _ = WebviewWindowBuilder::new(&app, "embedded_browser", WebviewUrl::External(url.parse().unwrap()))
        .title("Browser")
        .decorations(false)
        .skip_taskbar(true)
        .resizable(false)
        .parent(&main_window).unwrap()
        .inner_size(webview_w, webview_h)
        .position(webview_x, webview_y)
        .initialization_script(&init_script)
        .on_navigation(move |url: &Url| {
             let url_str = url.as_str();
             if url_str.starts_with("https://nsl.local/save/") {
                 let parts: Vec<&str> = url_str.split('/').collect();
                 if parts.len() >= 6 {
                     let u_res = general_purpose::STANDARD.decode(parts[4]);
                     let p_res = general_purpose::STANDARD.decode(parts[5]);
                     if let (Ok(u), Ok(p)) = (u_res, p_res) {
                         // Dùng domain ảo tạm thời để test logic save, hoặc parse từ url gốc
                         // Ở đây ta gọi hàm perform_save_account mà trước đó bị lỗi thiếu
                         let _ = perform_save_account(
                             &app_handle_clone, 
                             "detected_login".to_string(), // Tạm thời dùng key này hoặc cần logic lấy domain
                             String::from_utf8(u).unwrap(), 
                             String::from_utf8(p).unwrap()
                         );
                     }
                 }
                 return false;
             }
             true
        })
        .build();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            save_account, 
            get_all_accounts, 
            get_password_plaintext, 
            delete_account,
            open_secure_window, 
            navigate_webview, 
            hide_embedded_view, 
            update_webview_layout
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}