use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, LogicalPosition, LogicalSize, Url};
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde::{Deserialize, Serialize};
use base64::{engine::general_purpose, Engine as _};

// --- CẤU TRÚC DỮ LIỆU (4 TRƯỜNG) ---
// Key: Domain -> Value: (User, Pass_Encrypted, Cap, Truong)
#[derive(Serialize, Deserialize, Debug, Clone)]
struct AccountStore {
    accounts: HashMap<String, (String, String, String, String)>,
}

#[derive(Serialize)]
struct AccountDTO {
    domain: String,
    username: String,
    cap: String,
    truong: String,
}

const SECRET_KEY: &str = "NSL_SECURE_KEY_2026_HCM"; 

// --- HELPER FUNCTIONS ---
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

fn perform_save_account(app: &AppHandle, domain: String, user: String, pass: String, cap: String, truong: String) -> Result<String, String> {
    let mut store = load_store(app);
    let mc = new_magic_crypt!(SECRET_KEY, 256);
    let encrypted_pass = mc.encrypt_str_to_base64(&pass);
    
    // Lưu dữ liệu (Ghi đè nếu đã tồn tại)
    store.accounts.insert(domain, (user, encrypted_pass, cap, truong));
    save_store(app, &store)?;
    Ok("Đã lưu thành công!".to_string())
}

// --- COMMANDS ---

#[tauri::command]
fn get_all_accounts(app: AppHandle) -> Vec<AccountDTO> {
    let store = load_store(&app);
    let mut list: Vec<AccountDTO> = store.accounts.iter().map(|(k, v)| {
        AccountDTO { 
            domain: k.clone(), 
            username: v.0.clone(),
            cap: v.2.clone(),   // Trường thứ 3 trong tuple
            truong: v.3.clone() // Trường thứ 4 trong tuple
        }
    }).collect();
    list.sort_by(|a, b| a.domain.cmp(&b.domain));
    list
}

// Hàm lấy chi tiết để hiển thị lên Modal Sửa (trả về mảng string)
#[tauri::command]
fn get_full_account_details(app: AppHandle, domain: String) -> Result<Vec<String>, String> {
    let store = load_store(&app);
    if let Some(data) = store.accounts.get(&domain) {
        let mc = new_magic_crypt!(SECRET_KEY, 256);
        let pass_dec = mc.decrypt_base64_to_string(&data.1).unwrap_or_default();
        // Trả về: [User, Pass, Cap, Truong]
        return Ok(vec![data.0.clone(), pass_dec, data.2.clone(), data.3.clone()]);
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
fn save_account(app: AppHandle, domain: String, user: String, pass: String, cap: String, truong: String) -> Result<String, String> {
    perform_save_account(&app, domain, user, pass, cap, truong)
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
    if let Some(win) = app.get_webview_window("embedded_browser") { let _ = win.close(); }
}

#[tauri::command]
async fn navigate_webview(app: AppHandle, url: String) {
    if let Some(win) = app.get_webview_window("embedded_browser") {
        let script = format!("window.location.replace('{}')", url);
        let _ = win.eval(&script);
    }
}

// --- LOGIC OPEN WINDOW & INJECTOR (FULL TELERIK SUPPORT) ---
#[tauri::command]
async fn open_secure_window(app: AppHandle, url: String) {
    let domain_raw = url.replace("https://", "").replace("http://", "");
    let domain = domain_raw.split('/').next().unwrap_or("").to_string();
    
    let store = load_store(&app);
    let mut u_val = String::new();
    let mut p_val = String::new();
    let mut c_val = String::new();
    let mut t_val = String::new();

    if let Some(data) = store.accounts.get(&domain) {
        let mc = new_magic_crypt!(SECRET_KEY, 256);
        u_val = data.0.clone();
        p_val = mc.decrypt_base64_to_string(&data.1).unwrap_or_default();
        c_val = data.2.clone();
        t_val = data.3.clone();
    }

    // SCRIPT JS TIÊM VÀO TRANG WEB
    let init_script = format!(r#"
        window.addEventListener('DOMContentLoaded', () => {{
            console.log("🔥 NSL Auto-Fill Pro v6: Full Telerik Support");
            const tUser = "{}"; const tPass = "{}"; const tCap = "{}"; const tTruong = "{}";

            function checkAndFill() {{
                // 1. Tự Click Tab QLTH
                let spans = document.querySelectorAll('.rtsTxt');
                for (let span of spans) {{
                    if (span.innerText.trim() === "Tài khoản QLTH") {{
                        let link = span.closest('a.rtsLink');
                        if (link && !link.classList.contains('rtsSelected')) link.click();
                        break;
                    }}
                }}

                // 2. Điền User & Pass
                if (tUser) {{
                    let uIn = document.getElementById('ContentPlaceHolder1_tbU') || document.querySelector('input[name$="tbU"]');
                    let pIn = document.getElementById('ContentPlaceHolder1_tbP') || document.querySelector('input[name$="tbP"]');
                    if (uIn && pIn && uIn.value !== tUser) {{
                        uIn.value = tUser; pIn.value = tPass;
                        [uIn, pIn].forEach(el => {{
                            el.dispatchEvent(new Event('input', {{bubbles:true}}));
                            el.dispatchEvent(new Event('change', {{bubbles:true}}));
                            el.dispatchEvent(new Event('blur', {{bubbles:true}}));
                        }});
                    }}
                }}

                // 3. Điền CẤP & TRƯỜNG (Xử lý input Telerik)
                if (tCap || tTruong) {{
                    let capIn = document.getElementById('ctl00_ContentPlaceHolder1_cbCapHoc_Input');
                    let trgIn = document.getElementById('ctl00_ContentPlaceHolder1_cbTruong_Input');
                    
                    if (capIn && tCap && capIn.value !== tCap) {{
                        capIn.value = tCap;
                        capIn.dispatchEvent(new Event('input', {{bubbles:true}}));
                        // Telerik cần focus/blur để nhận giá trị
                        capIn.focus(); 
                        capIn.blur();
                    }}
                    
                    if (trgIn && tTruong && trgIn.value !== tTruong) {{
                        trgIn.value = tTruong;
                        trgIn.dispatchEvent(new Event('input', {{bubbles:true}}));
                        trgIn.focus();
                        trgIn.blur();
                    }}
                }}
            }}

            function setupCapture() {{
                function sendToRust() {{
                    let u = document.getElementById('ContentPlaceHolder1_tbU');
                    let p = document.getElementById('ContentPlaceHolder1_tbP');
                    let c = document.getElementById('ctl00_ContentPlaceHolder1_cbCapHoc_Input');
                    let t = document.getElementById('ctl00_ContentPlaceHolder1_cbTruong_Input');
                    
                    if (u && p && u.value && p.value) {{
                        let cVal = c ? c.value : "";
                        let tVal = t ? t.value : "";

                        // Encode Base64
                        let u64 = btoa(unescape(encodeURIComponent(u.value)));
                        let p64 = btoa(unescape(encodeURIComponent(p.value)));
                        let c64 = btoa(unescape(encodeURIComponent(cVal)));
                        let t64 = btoa(unescape(encodeURIComponent(tVal)));
                        
                        // Gửi qua Iframe ẩn
                        let iframe = document.createElement('iframe');
                        iframe.style.display = 'none';
                        iframe.src = "https://nsl.local/save/" + u64 + "/" + p64 + "/" + c64 + "/" + t64;
                        document.body.appendChild(iframe);
                        setTimeout(() => document.body.removeChild(iframe), 1000);
                    }}
                }}

                // Gắn sự kiện vào nút Đăng nhập
                let btn = document.getElementById('ContentPlaceHolder1_btOK');
                if (btn && !btn.hasAttribute('data-captured')) {{
                    btn.setAttribute('data-captured', 'true');
                    // Dùng mousedown để bắt trước khi form submit
                    btn.addEventListener('mousedown', sendToRust);
                }}
                
                // Gắn sự kiện Enter vào các ô input quan trọng
                let inputs = [
                    document.getElementById('ContentPlaceHolder1_tbP'), 
                    document.getElementById('ctl00_ContentPlaceHolder1_cbTruong_Input')
                ];
                inputs.forEach(inp => {{
                    if(inp && !inp.hasAttribute('data-captured')) {{
                        inp.setAttribute('data-captured', 'true');
                        inp.addEventListener('keydown', (e) => {{ if(e.key==='Enter') sendToRust(); }});
                    }}
                }});
            }}

            const observer = new MutationObserver(() => {{ checkAndFill(); setupCapture(); }});
            observer.observe(document.body, {{ childList: true, subtree: true }});
            checkAndFill();
        }});
    "#, u_val, p_val, c_val, t_val);

    if let Some(win) = app.get_webview_window("embedded_browser") { let _ = win.close(); }
    let main_window = app.get_webview_window("main").unwrap();
    let size = main_window.inner_size().unwrap();
    let webview_x = 260.0; let webview_y = 64.0;
    let webview_w = (size.width as f64) - webview_x; let webview_h = (size.height as f64) - webview_y;
    let app_handle_clone = app.clone();
    let target_domain = domain.clone();

    let _ = WebviewWindowBuilder::new(&app, "embedded_browser", WebviewUrl::External(url.parse().unwrap()))
        .title("Browser").decorations(false).skip_taskbar(true).resizable(false).parent(&main_window).unwrap()
        .inner_size(webview_w, webview_h).position(webview_x, webview_y)
        .initialization_script(&init_script)
        .on_navigation(move |url: &Url| {
             let url_str = url.as_str();
             if url_str.starts_with("https://nsl.local/save/") {
                 let parts: Vec<&str> = url_str.split('/').collect();
                 // Cấu trúc URL: /save/u/p/c/t -> cần 8 phần tử
                 if parts.len() >= 8 {
                     let u = String::from_utf8(general_purpose::STANDARD.decode(parts[4]).unwrap_or_default()).unwrap_or_default();
                     let p = String::from_utf8(general_purpose::STANDARD.decode(parts[5]).unwrap_or_default()).unwrap_or_default();
                     let c = String::from_utf8(general_purpose::STANDARD.decode(parts[6]).unwrap_or_default()).unwrap_or_default();
                     let t = String::from_utf8(general_purpose::STANDARD.decode(parts[7]).unwrap_or_default()).unwrap_or_default();
                     
                     let _ = perform_save_account(&app_handle_clone, target_domain.clone(), u, p, c, t);
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
            save_account, get_all_accounts, get_full_account_details, delete_account,
            open_secure_window, navigate_webview, hide_embedded_view, update_webview_layout
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}