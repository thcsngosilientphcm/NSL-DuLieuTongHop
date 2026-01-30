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
    
    // Ghi đè thông tin cũ
    store.accounts.insert(domain, (user, encrypted_pass, cap, truong));
    save_store(app, &store)?;
    Ok("Đã lưu thành công!".to_string())
}

// --- COMMANDS ---
#[tauri::command]
fn get_all_accounts(app: AppHandle) -> Vec<AccountDTO> {
    let store = load_store(&app);
    let mut list: Vec<AccountDTO> = store.accounts.iter().map(|(k, v)| {
        AccountDTO { domain: k.clone(), username: v.0.clone(), cap: v.2.clone(), truong: v.3.clone() }
    }).collect();
    list.sort_by(|a, b| a.domain.cmp(&b.domain));
    list
}

#[tauri::command]
fn get_full_account_details(app: AppHandle, domain: String) -> Result<Vec<String>, String> {
    let store = load_store(&app);
    if let Some(data) = store.accounts.get(&domain) {
        let mc = new_magic_crypt!(SECRET_KEY, 256);
        let pass_dec = mc.decrypt_base64_to_string(&data.1).unwrap_or_default();
        return Ok(vec![data.0.clone(), pass_dec, data.2.clone(), data.3.clone()]);
    }
    Err("Không tìm thấy".to_string())
}

#[tauri::command]
fn delete_account(app: AppHandle, domain: String) -> Result<String, String> {
    let mut store = load_store(&app);
    if store.accounts.remove(&domain).is_some() {
        save_store(&app, &store)?; Ok("Đã xóa".to_string())
    } else { Err("Không tồn tại".to_string()) }
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

// --- LOGIC QUAN TRỌNG NHẤT: AUTO-FILL & CAPTURE ---
#[tauri::command]
async fn open_secure_window(app: AppHandle, url: String) {
    let domain_raw = url.replace("https://", "").replace("http://", "");
    let domain = domain_raw.split('/').next().unwrap_or("").to_string();
    
    let store = load_store(&app);
    let mut u_v = String::new(); let mut p_v = String::new();
    let mut c_v = String::new(); let mut t_v = String::new();

    if let Some(d) = store.accounts.get(&domain) {
        let mc = new_magic_crypt!(SECRET_KEY, 256);
        u_v = d.0.clone();
        p_v = mc.decrypt_base64_to_string(&d.1).unwrap_or_default();
        c_v = d.2.clone();
        t_v = d.3.clone();
    }

    let init_script = format!(r#"
        window.addEventListener('DOMContentLoaded', () => {{
            console.log("🔥 NSL Injector v8: Specific ID Targeting");
            
            // DỮ LIỆU CẦN ĐIỀN
            const data = {{
                u: "{}", p: "{}", c: "{}", t: "{}"
            }};

            // ID CHÍNH XÁC CỦA TRANG WEB
            const IDS = {{
                user: "ContentPlaceHolder1_tbU",
                pass: "ContentPlaceHolder1_tbP",
                cap: "ctl00_ContentPlaceHolder1_cbCapHoc_Input",
                truong: "ctl00_ContentPlaceHolder1_cbTruong_Input",
                btn: "ContentPlaceHolder1_btOK"
            }};

            // 1. HÀM ĐIỀN DỮ LIỆU (Chạy liên tục để đảm bảo điền được)
            function autoFill() {{
                // Click Tab nếu chưa chọn
                let spans = document.querySelectorAll('.rtsTxt');
                for (let s of spans) {{
                    if (s.innerText.trim() === "Tài khoản QLTH") {{
                        let link = s.closest('a.rtsLink');
                        if (link && !link.classList.contains('rtsSelected')) link.click();
                        break;
                    }}
                }}

                if (!data.u) return;

                // Hàm nhỏ để set giá trị và kích hoạt sự kiện
                const setVal = (id, val) => {{
                    let el = document.getElementById(id);
                    if (el && el.value !== val) {{
                        el.value = val;
                        el.dispatchEvent(new Event('input', {{bubbles:true}}));
                        el.dispatchEvent(new Event('change', {{bubbles:true}}));
                        el.dispatchEvent(new Event('blur', {{bubbles:true}})); // Quan trọng cho Telerik
                    }}
                }};

                setVal(IDS.user, data.u);
                setVal(IDS.pass, data.p);
                if(data.c) setVal(IDS.cap, data.c);
                if(data.t) setVal(IDS.truong, data.t);
            }}

            // 2. HÀM BẮT SỰ KIỆN ĐĂNG NHẬP (Chặn nút bấm)
            function setupCapture() {{
                let btn = document.getElementById(IDS.btn);
                
                // Nếu tìm thấy nút và chưa gắn sự kiện
                if (btn && !btn.hasAttribute('data-nsl-hook')) {{
                    btn.setAttribute('data-nsl-hook', 'true');
                    
                    // GẮN SỰ KIỆN 'CLICK' Ở PHA CAPTURE (Để chặn trước khi WebForm chạy)
                    btn.addEventListener('click', function(e) {{
                        // Nếu là lệnh do chính tool gọi lại thì bỏ qua
                        if (this.getAttribute('data-processing') === 'true') return;

                        console.log(">> Đã chặn nút đăng nhập để lưu dữ liệu...");
                        e.preventDefault(); // Chặn WebForm
                        e.stopImmediatePropagation(); // Chặn các script khác

                        // Lấy dữ liệu hiện tại trên form
                        let u = document.getElementById(IDS.user)?.value || "";
                        let p = document.getElementById(IDS.pass)?.value || "";
                        let c = document.getElementById(IDS.cap)?.value || "";
                        let t = document.getElementById(IDS.truong)?.value || "";

                        if (u && p) {{
                            // Mã hóa
                            let u64 = btoa(unescape(encodeURIComponent(u)));
                            let p64 = btoa(unescape(encodeURIComponent(p)));
                            let c64 = btoa(unescape(encodeURIComponent(c)));
                            let t64 = btoa(unescape(encodeURIComponent(t)));

                            // Gửi về Rust bằng Iframe ẩn
                            let iframe = document.createElement('iframe');
                            iframe.style.display = 'none';
                            iframe.src = "https://nsl.local/save/" + u64 + "/" + p64 + "/" + c64 + "/" + t64;
                            document.body.appendChild(iframe);
                            
                            // Dọn dẹp iframe sau 1s
                            setTimeout(() => {{ if(iframe) iframe.remove(); }}, 1000);
                        }}

                        // SAU 0.5 GIÂY -> BẤM LẠI NÚT THẬT
                        let self = this;
                        self.setAttribute('data-processing', 'true');
                        setTimeout(() => {{
                            console.log(">> Tiếp tục đăng nhập...");
                            self.click(); // Kích hoạt sự kiện gốc
                            // Reset sau khi click để lần sau bắt tiếp (nếu đăng nhập sai)
                            setTimeout(() => self.removeAttribute('data-processing'), 2000); 
                        }}, 500);

                    }}, true); // <--- QUAN TRỌNG: UseCapture = true
                }}
            }}

            // CHẠY LIÊN TỤC (Giám sát DOM)
            const observer = new MutationObserver(() => {{
                autoFill();
                setupCapture();
            }});
            observer.observe(document.body, {{ childList: true, subtree: true }});
            
            // Chạy ngay lập tức
            autoFill();
            setupCapture();
        }});
    "#, u_v, p_v, c_v, t_v);

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
                 // Cấu trúc: /save/u/p/c/t
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