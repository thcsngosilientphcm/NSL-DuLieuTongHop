use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, LogicalPosition, LogicalSize, Url};
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde::{Deserialize, Serialize};
use base64::{engine::general_purpose, Engine as _};

// --- DATA STRUCTURES ---
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
    if let Ok(store) = serde_json::from_str::<AccountStore>(&data) { return store; }
    
    // Migration Logic
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

// --- INJECTOR V22: CORRECT TAB LOGIC + TELERIK SELECTION ---
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
            console.log("🔥 NSL V22: Correct Tab + Telerik Select");
            const accounts = {}; 
            const IDS = {{
                user: "ContentPlaceHolder1_tbU",
                pass: "ContentPlaceHolder1_tbP",
                // Lưu ý: ID cho Telerik Object không có đuôi _Input
                cap: "ctl00_ContentPlaceHolder1_cbCapHoc", 
                truong: "ctl00_ContentPlaceHolder1_cbTruong",
                btn: "ContentPlaceHolder1_btOK"
            }};

            // 1. CHUYỂN TAB (BẮT BUỘC PHẢI CÓ)
            function ensureTabSelected() {{
                let spans = document.querySelectorAll('.rtsTxt');
                for (let s of spans) {{
                    if (s.innerText.trim() === "Tài khoản QLTH") {{
                        let link = s.closest('a.rtsLink');
                        if (link && !link.classList.contains('rtsSelected')) {{
                            console.log(">> Clicking Tab QLTH");
                            link.click();
                            return true; // Đã click, cần chờ load
                        }}
                    }}
                }}
                return false; // Đã ở đúng tab hoặc không tìm thấy
            }}

            // 2. HÀM ĐIỀN DỮ LIỆU CHUẨN (User/Pass: DOM, Cấp/Trường: Telerik Select)
            function smartFill(acc) {{
                if (!acc) return;
                
                // A. User & Pass (Dùng DOM như cũ - bạn xác nhận cách này ok)
                let uEl = document.getElementById(IDS.user);
                let pEl = document.getElementById(IDS.pass);
                if (uEl) {{ uEl.value = acc.u; uEl.dispatchEvent(new Event('input', {{bubbles:true}})); }}
                if (pEl) {{ pEl.value = acc.p; pEl.dispatchEvent(new Event('input', {{bubbles:true}})); }}

                // B. Cấp & Trường (PHẢI DÙNG TELERIK API ĐỂ KHÔNG BỊ LỖI)
                // Nếu chỉ điền DOM, server sẽ không nhận được ID và báo sai.
                if (typeof $find !== 'undefined') {{
                    
                    // Điền Cấp trước
                    let comboCap = $find(IDS.cap);
                    if (comboCap && acc.c) {{
                        // Tìm Item trong danh sách có chữ khớp với dữ liệu
                        let item = comboCap.findItemByText(acc.c);
                        if (item) {{
                            item.select(); // CHÌA KHÓA: Chọn item sẽ tự điền text và set value ẩn
                        }} else {{
                            comboCap.set_text(acc.c); // Fallback
                        }}
                    }}

                    // Delay 1 chút để Cấp load xong mới điền Trường
                    setTimeout(() => {{
                        let comboTruong = $find(IDS.truong);
                        if (comboTruong && acc.t) {{
                            let item = comboTruong.findItemByText(acc.t);
                            if (item) {{
                                item.select(); // CHÌA KHÓA: Chọn item chuẩn
                            }} else {{
                                comboTruong.set_text(acc.t);
                            }}
                        }}
                    }}, 800); // Đợi 800ms cho AJAX tải danh sách trường
                }}
            }}

            // 3. MENU CHỌN TÀI KHOẢN (UI)
            function showMenu() {{
                let old = document.getElementById('nsl-menu-overlay'); if (old) old.remove();
                let uIn = document.getElementById(IDS.user); if (!uIn) return;

                let div = document.createElement('div');
                div.id = 'nsl-menu-overlay';
                div.style.cssText = 'position:absolute;z-index:9999999;background:#1e293b;border:1px solid #475569;border-radius:6px;box-shadow:0 10px 25px rgba(0,0,0,0.5);padding:6px;min-width:250px;color:white;font-family:sans-serif;font-size:13px;';
                div.innerHTML = '<div style="color:#94a3b8;padding:4px 8px;border-bottom:1px solid #334155;margin-bottom:4px;font-weight:bold;">Chọn tài khoản:</div>';

                accounts.forEach(acc => {{
                    let item = document.createElement('div');
                    item.innerHTML = `<span style="color:#22d3ee;font-weight:bold">${{acc.u}}</span>${{acc.t ? '<br><span style="color:#94a3b8;font-size:11px">'+acc.t+'</span>' : ''}}`;
                    item.style.cssText = 'padding:6px 10px;cursor:pointer;border-radius:4px;margin-bottom:2px;';
                    item.onmouseover = () => item.style.background = '#334155';
                    item.onmouseout = () => item.style.background = 'transparent';
                    item.onmousedown = (e) => {{ // Dùng mousedown để ưu tiên hơn blur
                        e.preventDefault(); e.stopPropagation();
                        smartFill(acc);
                        div.remove();
                    }};
                    div.appendChild(item);
                }});

                let rect = uIn.getBoundingClientRect();
                div.style.top = (rect.bottom + window.scrollY + 2) + 'px';
                div.style.left = (rect.left + window.scrollX) + 'px';
                document.body.appendChild(div);

                // Click ra ngoài thì đóng
                setTimeout(() => {{
                    document.addEventListener('click', function close(e) {{
                        if (!div.contains(e.target) && e.target !== uIn) {{
                            div.remove(); document.removeEventListener('click', close);
                        }}
                    }});
                }}, 100);
            }}

            // 4. LOGIC KHỞI CHẠY (LOOP CHECK)
            let initialized = false;
            
            function mainLoop() {{
                // A. Luôn kiểm tra và click Tab nếu chưa đúng
                ensureTabSelected();

                let uIn = document.getElementById(IDS.user);
                if (uIn) {{
                    // B. Gắn sự kiện click vào ô User để hiện menu
                    if (!uIn.dataset.hooked) {{
                        uIn.dataset.hooked = "true";
                        uIn.addEventListener('click', (e) => {{ e.stopPropagation(); showMenu(); }});
                    }}

                    // C. Auto-fill lần đầu tiên (nếu có TK và ô đang trống)
                    if (!initialized && accounts.length > 0 && !uIn.value) {{
                        console.log(">> First load auto-fill");
                        // Delay nhẹ 300ms để chắc chắn Telerik đã load xong script
                        setTimeout(() => smartFill(accounts[0]), 300);
                        initialized = true;
                    }}
                }}
            }}

            // 5. CAPTURE DATA (ĐỂ LƯU MỚI)
            function attachCapture() {{
                const btn = document.getElementById(IDS.btn);
                if (btn && !btn.dataset.captured) {{
                    btn.dataset.captured = "true";
                    btn.addEventListener('mousedown', () => {{
                        let u = document.getElementById(IDS.user)?.value || "";
                        let p = document.getElementById(IDS.pass)?.value || "";
                        let c = "", t = "";
                        
                        // Lấy text hiển thị từ Telerik để lưu cho đẹp
                        if(typeof $find !== 'undefined') {{
                            c = $find(IDS.cap)?.get_text() || "";
                            t = $find(IDS.truong)?.get_text() || "";
                        }}
                        // Fallback DOM
                        if(!c) c = document.getElementById(IDS.cap + "_Input")?.value || "";
                        if(!t) t = document.getElementById(IDS.truong + "_Input")?.value || "";

                        if (u && p) {{
                            let base = "https://nsl.local/save/";
                            let parts = [u, p, c, t].map(s => btoa(unescape(encodeURIComponent(s))));
                            new Image().src = base + parts.join("/");
                        }}
                    }});
                }}
            }}

            // Chạy loop mỗi 500ms để đảm bảo Tab luôn đúng và Element đã load
            setInterval(() => {{ mainLoop(); attachCapture(); }}, 500);

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