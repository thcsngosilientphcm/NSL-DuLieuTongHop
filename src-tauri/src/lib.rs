use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewBuilder, LogicalPosition, LogicalSize, Url}; // Thêm Url
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde::{Deserialize, Serialize};
use base64::{engine::general_purpose, Engine as _};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AccountStore {
    accounts: HashMap<String, (String, String)>,
}

const SECRET_KEY: &str = "NSL_SECURE_KEY_2026_HCM"; 

fn get_creds_path(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap().join("creds.json")
}

// --- HÀM LƯU DỮ LIỆU ---
fn perform_save_account(app: &AppHandle, domain: String, user: String, pass: String) -> Result<String, String> {
    let path = get_creds_path(app);
    if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }

    let mut store = if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or(AccountStore { accounts: HashMap::new() })
    } else {
        AccountStore { accounts: HashMap::new() }
    };

    let mc = new_magic_crypt!(SECRET_KEY, 256);
    
    // Kiểm tra trùng lặp
    if let Some((stored_user, stored_pass_enc)) = store.accounts.get(&domain) {
        if stored_user == &user {
            if let Ok(stored_pass_dec) = mc.decrypt_base64_to_string(stored_pass_enc) {
                if stored_pass_dec == pass {
                    return Ok("Dữ liệu không đổi".to_string());
                }
            }
        }
    }

    if !user.trim().is_empty() && !pass.trim().is_empty() {
        let encrypted_pass = mc.encrypt_str_to_base64(&pass);
        store.accounts.insert(domain.clone(), (user, encrypted_pass));
        let json = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())?;
        return Ok("Đã lưu thành công!".to_string());
    }
    Err("Dữ liệu rỗng".to_string())
}

#[tauri::command]
fn save_account(app: AppHandle, domain: String, user: String, pass: String) -> Result<String, String> {
    perform_save_account(&app, domain, user, pass)
}

// --- LỆNH 1: CẬP NHẬT KÍCH THƯỚC WEBVIEW (KHI RESIZE SIDEBAR) ---
#[tauri::command]
fn update_webview_layout(app: AppHandle, sidebar_width: f64) {
    if let Some(view) = app.get_webview("embedded_browser") {
        if let Some(main_window) = app.get_webview_window("main") {
            let size = main_window.inner_size().unwrap();
            let header_height = 64.0;
            
            let total_width = size.width as f64;
            let total_height = size.height as f64;

            let webview_x = sidebar_width;
            let webview_y = header_height;
            let webview_w = total_width - sidebar_width;
            let webview_h = total_height - header_height;

            let _ = view.set_bounds(tauri::Rect {
                position: LogicalPosition::new(webview_x, webview_y).into(),
                size: LogicalSize::new(webview_w, webview_h).into(),
            });
        }
    }
}

// --- LỆNH 2: ĐIỀU HƯỚNG (FIX LỖI load_url) ---
#[tauri::command]
async fn navigate_webview(app: AppHandle, url: String) {
    if let Some(view) = app.get_webview("embedded_browser") {
        // Dùng eval JS để điều hướng thay vì load_url (tránh lỗi API thay đổi)
        let script = format!("window.location.replace('{}')", url);
        let _ = view.eval(&script);
    }
}

// --- LỆNH 3: ẨN WEBVIEW (KHI UPDATE) ---
#[tauri::command]
fn hide_embedded_view(app: AppHandle) {
    if let Some(view) = app.get_webview("embedded_browser") {
        // Có thể dùng close() hoặc hide(). Hide sẽ giữ trạng thái web.
        let _ = view.close(); 
    }
}

// --- LỆNH 4: MỞ WEBVIEW LỒNG GHÉP (FIX LỖI BOUNDS & TYPE) ---
#[tauri::command]
async fn open_secure_window(app: AppHandle, url: String) {
    let domain_raw = url.replace("https://", "").replace("http://", "");
    let domain = domain_raw.split('/').next().unwrap_or("").to_string();
    
    let path = get_creds_path(&app);
    let mut username = String::new();
    let mut password = String::new();

    if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        if let Ok(store) = serde_json::from_str::<AccountStore>(&data) {
            if let Some((u, p_enc)) = store.accounts.get(&domain) {
                let mc = new_magic_crypt!(SECRET_KEY, 256);
                if let Ok(p_dec) = mc.decrypt_base64_to_string(p_enc) {
                    username = u.clone();
                    password = p_dec;
                }
            }
        }
    }

    // Script Tiêm vào web
    let init_script = format!(r#"
        window.addEventListener('DOMContentLoaded', () => {{
            console.log("🔥 NSL Embedded v3 Active");

            function autoClickTab() {{
                let spans = document.querySelectorAll('.rtsTxt');
                for (let span of spans) {{
                    if (span.innerText.trim() === "Tài khoản QLTH") {{
                        let link = span.closest('a.rtsLink');
                        if (link) link.click();
                        return;
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
            setTimeout(autoClickTab, 1500);
            setTimeout(autoFill, 800);
            setupCapture();
        }});
    "#, username, password);

    // Xử lý Webview cũ
    if let Some(view) = app.get_webview("embedded_browser") {
        let _ = view.close();
    }

    let main_window = app.get_webview_window("main").unwrap();
    let app_handle_clone = app.clone();
    let domain_clone = domain.clone();

    // 1. TẠO WEBVIEW (Không set bounds ở đây để tránh lỗi)
    let webview = WebviewBuilder::new("embedded_browser", WebviewUrl::External(url.parse().unwrap()))
        .auto_resize()
        .initialization_script(&init_script)
        // FIX LỖI TYPE: Thêm kiểu dữ liệu cho biến url
        .on_navigation(move |url: &Url| {
             let url_str = url.as_str();
             if url_str.starts_with("https://nsl.local/save/") {
                 let parts: Vec<&str> = url_str.split('/').collect();
                 if parts.len() >= 6 {
                     let u_res = general_purpose::STANDARD.decode(parts[4]);
                     let p_res = general_purpose::STANDARD.decode(parts[5]);
                     if let (Ok(u), Ok(p)) = (u_res, p_res) {
                         let _ = perform_save_account(&app_handle_clone, domain_clone.clone(), String::from_utf8(u).unwrap(), String::from_utf8(p).unwrap());
                     }
                 }
                 return false;
             }
             true
        })
        .build(&main_window); // Tạo xong webview

    // 2. SET BOUNDS SAU KHI TẠO (FIX LỖI BOUNDS)
    if let Ok(view) = webview {
        let size = main_window.inner_size().unwrap();
        
        let webview_x = 260.0;
        let webview_y = 64.0;
        let webview_w = (size.width as f64) - webview_x;
        let webview_h = (size.height as f64) - webview_y;
        
        let _ = view.set_bounds(tauri::Rect {
            position: LogicalPosition::new(webview_x, webview_y).into(),
            size: LogicalSize::new(webview_w, webview_h).into(),
        });
    }
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
            open_secure_window, 
            navigate_webview, 
            hide_embedded_view,
            update_webview_layout 
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}