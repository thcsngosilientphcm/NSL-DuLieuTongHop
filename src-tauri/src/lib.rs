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

// --- CÁC HÀM HELPER ---

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

fn perform_save_account(app: &AppHandle, domain: String, user: String, pass: String) -> Result<String, String> {
    let mut store = load_store(app);
    let mc = new_magic_crypt!(SECRET_KEY, 256);
    
    // Kiểm tra trùng lặp: Nếu User và Pass y hệt cũ thì không lưu lại (để tránh ghi file nhiều lần)
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

// --- CÁC COMMAND (Giao tiếp với JS) ---

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
            let _ = win.set_size(LogicalSize::new((size.width as f64) - sidebar_width, (size.height as f64) - header_height));
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

// --- LOGIC MỞ CỬA SỔ VÀ TIÊM SCRIPT THÔNG MINH ---
#[tauri::command]
async fn open_secure_window(app: AppHandle, url: String) {
    // 1. Phân tích domain để lấy mật khẩu đã lưu
    let domain_raw = url.replace("https://", "").replace("http://", "");
    let domain = domain_raw.split('/').next().unwrap_or("").to_string();
    
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

    // --- SCRIPT QUAN SÁT VÀ TỰ ĐỘNG ĐIỀN (Dùng MutationObserver) ---
    // Script này sử dụng đúng ID của trang QLTH: ContentPlaceHolder1_tbU (User), ContentPlaceHolder1_tbP (Pass), ContentPlaceHolder1_btOK (Button)
    let init_script = format!(r#"
        window.addEventListener('DOMContentLoaded', () => {{
            console.log("🔥 NSL Auto-Fill v5: Targeting QLTH IDs");
            const targetUser = "{}";
            const targetPass = "{}";

            // HÀM 1: LIÊN TỤC QUAN SÁT VÀ ĐIỀN
            function checkAndFill() {{
                // A. Tự Click Tab
                let spans = document.querySelectorAll('.rtsTxt');
                for (let span of spans) {{
                    if (span.innerText.trim() === "Tài khoản QLTH") {{
                        let link = span.closest('a.rtsLink');
                        // Chỉ click nếu chưa được chọn (tránh reload trang liên tục)
                        if (link && !link.classList.contains('rtsSelected')) {{
                            console.log(">> Đã click Tab QLTH");
                            link.click();
                        }}
                        break; 
                    }}
                }}

                // B. Tự Điền Mật Khẩu (Dùng đúng ID)
                if (targetUser) {{
                    let uInput = document.getElementById('ContentPlaceHolder1_tbU') || document.querySelector('input[name$="tbU"]');
                    let pInput = document.getElementById('ContentPlaceHolder1_tbP') || document.querySelector('input[name$="tbP"]');

                    if (uInput && pInput) {{
                        // Chỉ điền khi ô đang trống
                        if (uInput.value !== targetUser) {{
                            console.log(">> Đã tìm thấy ô nhập liệu -> Đang điền...");
                            uInput.value = targetUser;
                            pInput.value = targetPass;
                            
                            // Bắn sự kiện để ASP.NET biết đã có dữ liệu (Rất quan trọng)
                            uInput.dispatchEvent(new Event('input', {{bubbles:true}}));
                            uInput.dispatchEvent(new Event('change', {{bubbles:true}}));
                            pInput.dispatchEvent(new Event('input', {{bubbles:true}}));
                            pInput.dispatchEvent(new Event('change', {{bubbles:true}}));
                        }}
                    }}
                }}
            }}

            // HÀM 2: BẮT SỰ KIỆN ĐĂNG NHẬP (Dùng Iframe ẩn để không chặn Login)
            function setupCapture() {{
                function sendToRust() {{
                    let uInput = document.getElementById('ContentPlaceHolder1_tbU');
                    let pInput = document.getElementById('ContentPlaceHolder1_tbP');
                    
                    if (uInput && pInput && uInput.value && pInput.value) {{
                        let u64 = btoa(unescape(encodeURIComponent(uInput.value)));
                        let p64 = btoa(unescape(encodeURIComponent(pInput.value)));
                        
                        // KỸ THUẬT IFRAME: Gửi tin ngầm, không làm chuyển trang chính
                        let iframe = document.createElement('iframe');
                        iframe.style.display = 'none';
                        iframe.src = "https://nsl.local/save/" + u64 + "/" + p64;
                        document.body.appendChild(iframe);
                        
                        // Xóa iframe sau 1 giây
                        setTimeout(() => document.body.removeChild(iframe), 1000);
                    }}
                }}

                // Gắn sự kiện vào ô Mật khẩu (Enter)
                let pInput = document.getElementById('ContentPlaceHolder1_tbP');
                if (pInput && !pInput.hasAttribute('data-captured')) {{
                    pInput.setAttribute('data-captured', 'true');
                    pInput.addEventListener('keydown', (e) => {{ if(e.key==='Enter') sendToRust(); }});
                }}

                // Gắn sự kiện vào Nút Đăng nhập (ID chuẩn: ContentPlaceHolder1_btOK)
                let btn = document.getElementById('ContentPlaceHolder1_btOK');
                if (btn && !btn.hasAttribute('data-captured')) {{
                    btn.setAttribute('data-captured', 'true');
                    // Dùng mousedown để bắt sớm hơn onclick của ASP.NET
                    btn.addEventListener('mousedown', () => sendToRust()); 
                }}
            }}

            // SỬ DỤNG MUTATION OBSERVER ĐỂ CANH CHỪNG 24/7
            // Bất cứ khi nào trang web hiện ra ô nhập liệu, nó sẽ điền ngay lập tức
            const observer = new MutationObserver((mutations) => {{
                checkAndFill();
                setupCapture();
            }});
            
            observer.observe(document.body, {{ childList: true, subtree: true }});
            
            // Chạy ngay lần đầu
            checkAndFill();
        }});
    "#, username, password);

    if let Some(win) = app.get_webview_window("embedded_browser") { let _ = win.close(); }
    
    let main_window = app.get_webview_window("main").unwrap();
    let size = main_window.inner_size().unwrap();
    
    let webview_x = 260.0;
    let webview_y = 64.0;
    let webview_w = (size.width as f64) - webview_x;
    let webview_h = (size.height as f64) - webview_y;
    let app_handle_clone = app.clone();
    let target_domain = domain.clone();

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
             // BẮT LINK ẢO TỪ IFRAME
             if url_str.starts_with("https://nsl.local/save/") {
                 let parts: Vec<&str> = url_str.split('/').collect();
                 if parts.len() >= 6 {
                     let u_res = general_purpose::STANDARD.decode(parts[4]);
                     let p_res = general_purpose::STANDARD.decode(parts[5]);
                     if let (Ok(u), Ok(p)) = (u_res, p_res) {
                         // Lưu với domain chính xác
                         let _ = perform_save_account(
                             &app_handle_clone, 
                             target_domain.clone(), 
                             String::from_utf8(u).unwrap(), 
                             String::from_utf8(p).unwrap()
                         );
                     }
                 }
                 return false; // Chặn iframe, không ảnh hưởng trang chính
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
            save_account, get_all_accounts, get_password_plaintext, delete_account,
            open_secure_window, navigate_webview, hide_embedded_view, update_webview_layout
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}