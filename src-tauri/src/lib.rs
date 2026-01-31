use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, LogicalPosition, LogicalSize, Url};
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde::{Deserialize, Serialize};
use base64::{engine::general_purpose, Engine as _};

// --- CẤU TRÚC DỮ LIỆU ---
#[derive(Serialize, Deserialize, Debug, Clone)]
struct AccountData {
    user: String,
    pass: String,
    cap: String,
    truong: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AccountStore {
    accounts: HashMap<String, Vec<AccountData>>,
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
    if !path.exists() { return AccountStore { accounts: HashMap::new() }; }
    
    let data = fs::read_to_string(&path).unwrap_or_default();
    
    // Đọc chuẩn
    if let Ok(store) = serde_json::from_str::<AccountStore>(&data) { return store; }
    
    // Nâng cấp dữ liệu cũ
    #[derive(Deserialize)] struct OldStore4 { accounts: HashMap<String, (String, String, String, String)> }
    #[derive(Deserialize)] struct OldStore2 { accounts: HashMap<String, (String, String)> }
    
    let mut new_map = HashMap::new();
    if let Ok(old4) = serde_json::from_str::<OldStore4>(&data) {
        for (d, (u, p, c, t)) in old4.accounts { new_map.insert(d, vec![AccountData{user:u, pass:p, cap:c, truong:t}]); }
    } else if let Ok(old2) = serde_json::from_str::<OldStore2>(&data) {
        for (d, (u, p)) in old2.accounts { new_map.insert(d, vec![AccountData{user:u, pass:p, cap:String::new(), truong:String::new()}]); }
    }
    
    let new_store = AccountStore { accounts: new_map };
    let _ = save_store(app, &new_store);
    new_store
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
    
    let new_acc = AccountData { user: user.clone(), pass: encrypted_pass, cap, truong };
    let list = store.accounts.entry(domain).or_insert(Vec::new());

    // Cập nhật nếu trùng User, ngược lại thêm mới
    if let Some(existing) = list.iter_mut().find(|a| a.user == user) {
        *existing = new_acc;
    } else {
        list.push(new_acc);
    }

    save_store(app, &store)?;
    Ok("Đã lưu thành công!".to_string())
}

// --- COMMANDS ---
#[tauri::command]
fn get_all_accounts(app: AppHandle) -> Vec<AccountDTO> {
    let store = load_store(&app);
    let mut list = Vec::new();
    for (d, accs) in store.accounts {
        for a in accs {
            list.push(AccountDTO { domain: d.clone(), username: a.user, cap: a.cap, truong: a.truong });
        }
    }
    list.sort_by(|a, b| a.domain.cmp(&b.domain).then(a.username.cmp(&b.username)));
    list
}

#[tauri::command]
fn get_full_account_details(app: AppHandle, domain: String, username: String) -> Result<Vec<String>, String> {
    let store = load_store(&app);
    if let Some(list) = store.accounts.get(&domain) {
        if let Some(acc) = list.iter().find(|a| a.user == username) {
            let mc = new_magic_crypt!(SECRET_KEY, 256);
            let p_dec = mc.decrypt_base64_to_string(&acc.pass).unwrap_or_default();
            return Ok(vec![acc.user.clone(), p_dec, acc.cap.clone(), acc.truong.clone()]);
        }
    }
    Err("Không tìm thấy".to_string())
}

#[tauri::command]
fn delete_account(app: AppHandle, domain: String, username: String) -> Result<String, String> {
    let mut store = load_store(&app);
    if let Some(list) = store.accounts.get_mut(&domain) {
        let before = list.len();
        list.retain(|a| a.user != username);
        if list.len() < before {
            if list.is_empty() { store.accounts.remove(&domain); }
            save_store(&app, &store)?;
            return Ok("Đã xóa".to_string());
        }
    }
    Err("Không tồn tại".to_string())
}

#[tauri::command]
fn save_account(app: AppHandle, domain: String, user: String, pass: String, cap: String, truong: String) -> Result<String, String> {
    perform_save_account(&app, domain, user, pass, cap, truong)
}

#[tauri::command]
fn update_webview_layout(app: AppHandle, sidebar_width: f64) {
    if let Some(win) = app.get_webview_window("embedded_browser") {
        if let Some(main) = app.get_webview_window("main") {
            let size = main.inner_size().unwrap();
            let _ = win.set_position(LogicalPosition::new(sidebar_width, 64.0));
            let _ = win.set_size(LogicalSize::new((size.width as f64) - sidebar_width, (size.height as f64) - 64.0));
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

// --- INJECTOR V15 (QUAY VỀ BẢN CHẤT - KHÔNG CAN THIỆP) ---
#[tauri::command]
async fn open_secure_window(app: AppHandle, url: String) {
    let domain_raw = url.replace("https://", "").replace("http://", "");
    let domain = domain_raw.split('/').next().unwrap_or("").to_string();
    
    let store = load_store(&app);
    let mut accounts_json = String::from("[]");

    if let Some(list) = store.accounts.get(&domain) {
        let mc = new_magic_crypt!(SECRET_KEY, 256);
        let mut items = Vec::new();
        for acc in list {
            let p_dec = mc.decrypt_base64_to_string(&acc.pass).unwrap_or_default();
            items.push(format!(r#"{{"u":"{}","p":"{}","c":"{}","t":"{}"}}"#, 
                acc.user.replace("\\", "\\\\").replace("\"", "\\\""), 
                p_dec.replace("\\", "\\\\").replace("\"", "\\\""), 
                acc.cap.replace("\\", "\\\\").replace("\"", "\\\""), 
                acc.truong.replace("\\", "\\\\").replace("\"", "\\\"")
            ));
        }
        accounts_json = format!("[{}]", items.join(","));
    }

    let init_script = format!(r#"
        window.addEventListener('DOMContentLoaded', () => {{
            console.log("🔥 NSL Native-Passive V15");
            const accounts = {}; 
            const IDS = {{
                user: "ContentPlaceHolder1_tbU",
                pass: "ContentPlaceHolder1_tbP",
                cap: "ctl00_ContentPlaceHolder1_cbCapHoc_Input",
                truong: "ctl00_ContentPlaceHolder1_cbTruong_Input",
                btn: "ContentPlaceHolder1_btOK"
            }};

            // 1. HÀM ĐIỀN (TUÂN THỦ QUY TRÌNH TELERIK: FOCUS -> NHẬP -> BLUR)
            function fillAccount(acc) {{
                if (!acc) return;
                const setVal = (id, val, force) => {{
                    let el = document.getElementById(id);
                    if (el) {{
                        // Nếu đang gõ phím thì không điền (tránh xung đột)
                        if (document.activeElement === el && !force) return;

                        // Chỉ điền khi ô trống hoặc có lệnh force
                        if (force || !el.value) {{
                            el.focus(); // Quan trọng cho Telerik
                            el.value = val;
                            el.dispatchEvent(new Event('input', {{bubbles:true}}));
                            el.dispatchEvent(new Event('change', {{bubbles:true}}));
                            el.blur();  // Quan trọng để trigger validation
                        }}
                    }}
                }};
                
                // Điền lần lượt
                setVal(IDS.user, acc.u, true);
                setVal(IDS.pass, acc.p, true);
                
                // Với Cấp và Trường, delay nhẹ 1 chút để đảm bảo UI kịp phản hồi
                if(acc.c) setTimeout(() => setVal(IDS.cap, acc.c, true), 100);
                if(acc.t) setTimeout(() => setVal(IDS.truong, acc.t, true), 200);
            }}

            // 2. MENU CHỌN TÀI KHOẢN (GIỮ NGUYÊN)
            function createAccountSelector(targetInput) {{
                let old = document.getElementById('nsl-acc-selector'); if(old) old.remove();
                let div = document.createElement('div');
                div.id = 'nsl-acc-selector';
                div.style.cssText = 'position:absolute;z-index:999999;background:#1e293b;border:1px solid #475569;border-radius:8px;box-shadow:0 10px 15px -3px rgba(0,0,0,0.5);padding:5px;min-width:200px;color:white;font-family:sans-serif;';
                
                let title = document.createElement('div');
                title.innerText = 'Chọn tài khoản:';
                title.style.cssText = 'font-size:12px;color:#94a3b8;padding:4px 8px;border-bottom:1px solid #334155;margin-bottom:4px;';
                div.appendChild(title);

                accounts.forEach(acc => {{
                    let item = document.createElement('div');
                    item.innerText = acc.u + (acc.t ? ' - ' + acc.t : '');
                    item.style.cssText = 'padding:8px 12px;cursor:pointer;font-size:14px;border-radius:4px;transition:background 0.2s;';
                    item.onmouseover = () => item.style.background = '#334155';
                    item.onmouseout = () => item.style.background = 'transparent';
                    item.onclick = (e) => {{ e.stopPropagation(); fillAccount(acc); div.remove(); }};
                    div.appendChild(item);
                }});

                let rect = targetInput.getBoundingClientRect();
                div.style.top = (rect.bottom + window.scrollY + 5) + 'px';
                div.style.left = (rect.left + window.scrollX) + 'px';
                div.style.width = rect.width + 'px';
                document.body.appendChild(div);
                
                const close = (e) => {{ if (!div.contains(e.target) && e.target !== targetInput) {{ div.remove(); document.removeEventListener('click', close); }} }};
                setTimeout(() => document.addEventListener('click', close), 100);
            }}

            // 3. LOGIC KHỞI TẠO (CHẠY 1 LẦN)
            let isInit = false;
            function initAutoFill() {{
                if(isInit) return;
                
                // Auto Click Tab
                let spans = document.querySelectorAll('.rtsTxt');
                for (let s of spans) {{
                    if (s.innerText.trim() === "Tài khoản QLTH") {{
                        let link = s.closest('a.rtsLink');
                        if (link && !link.classList.contains('rtsSelected')) link.click();
                        break;
                    }}
                }}

                let uIn = document.getElementById(IDS.user);
                if (uIn) {{
                    isInit = true;
                    uIn.setAttribute('data-nsl-init', 'true');
                    
                    if (accounts.length === 1 && !uIn.value) {{
                        fillAccount(accounts[0]);
                    }} else if (accounts.length > 0) {{
                        uIn.addEventListener('click', () => createAccountSelector(uIn));
                        uIn.addEventListener('focus', () => createAccountSelector(uIn));
                        if(!uIn.value) createAccountSelector(uIn);
                    }}
                }}
            }}

            // 4. BẮT SỰ KIỆN LƯU (PASSIVE MODE - TUYỆT ĐỐI KHÔNG CHẶN)
            // Ta dùng sự kiện 'mousedown' để lấy dữ liệu trước khi nút click được xử lý
            // Nhưng ta KHÔNG dùng preventDefault(), để trang web tự nhiên
            function attachCapture() {{
                let btn = document.getElementById(IDS.btn);
                if(btn && !btn.hasAttribute('data-nsl-capture')) {{
                    btn.setAttribute('data-nsl-capture', 'true');
                    
                    btn.addEventListener('mousedown', () => {{
                        let u = document.getElementById(IDS.user)?.value || "";
                        let p = document.getElementById(IDS.pass)?.value || "";
                        let c = document.getElementById(IDS.cap)?.value || "";
                        let t = document.getElementById(IDS.truong)?.value || "";

                        if (u && p) {{
                            console.log(">> NSL: Snapshot data taking...");
                            let u64 = btoa(unescape(encodeURIComponent(u)));
                            let p64 = btoa(unescape(encodeURIComponent(p)));
                            let c64 = btoa(unescape(encodeURIComponent(c)));
                            let t64 = btoa(unescape(encodeURIComponent(t)));
                            
                            // Dùng Image Ping: Gửi dữ liệu đi mà không quan tâm phản hồi
                            // Cách này đảm bảo 100% không ảnh hưởng luồng chính của web
                            new Image().src = "https://nsl.local/save/" + u64 + "/" + p64 + "/" + c64 + "/" + t64;
                        }}
                    }});
                }}
            }}

            // Chạy liên tục để bắt DOM load
            const obs = new MutationObserver(() => {{ initAutoFill(); attachCapture(); }});
            obs.observe(document.body, {{ childList: true, subtree: true }});
            // Chạy ngay
            setTimeout(() => {{ initAutoFill(); attachCapture(); }}, 500);
        }});
    "#, accounts_json);

    if let Some(win) = app.get_webview_window("embedded_browser") { let _ = win.close(); }
    let main = app.get_webview_window("main").unwrap();
    let size = main.inner_size().unwrap();
    let web_x = 260.0; let web_y = 64.0;
    let web_w = (size.width as f64) - web_x; let web_h = (size.height as f64) - web_y;
    let app_handle = app.clone();
    let domain_key = domain.clone();

    let _ = WebviewWindowBuilder::new(&app, "embedded_browser", WebviewUrl::External(url.parse().unwrap()))
        .title("Browser").decorations(false).skip_taskbar(true).resizable(false).parent(&main).unwrap()
        .inner_size(web_w, web_h).position(web_x, web_y)
        .initialization_script(&init_script)
        .on_navigation(move |url: &Url| {
             let s = url.as_str();
             if s.starts_with("https://nsl.local/save/") {
                 let p: Vec<&str> = s.split('/').collect();
                 if p.len() >= 8 {
                     let u = String::from_utf8(general_purpose::STANDARD.decode(p[4]).unwrap_or_default()).unwrap_or_default();
                     let pass = String::from_utf8(general_purpose::STANDARD.decode(p[5]).unwrap_or_default()).unwrap_or_default();
                     let c = String::from_utf8(general_purpose::STANDARD.decode(p[6]).unwrap_or_default()).unwrap_or_default();
                     let t = String::from_utf8(general_purpose::STANDARD.decode(p[7]).unwrap_or_default()).unwrap_or_default();
                     let _ = perform_save_account(&app_handle, domain_key.clone(), u, pass, c, t);
                 }
                 return false; // Chặn request ảo
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