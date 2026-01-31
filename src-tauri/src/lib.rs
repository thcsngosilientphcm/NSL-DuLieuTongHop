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

// --- INJECTOR V19 (TELERIK CORE API & AUTOFILL FIX) ---
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
            console.log("🔥 NSL Injector V19: Telerik Core API & Auto-Fill");
            const accounts = {}; 
            const IDS = {{
                user: "ContentPlaceHolder1_tbU",
                pass: "ContentPlaceHolder1_tbP",
                cap: "ctl00_ContentPlaceHolder1_cbCapHoc_Input",
                truong: "ctl00_ContentPlaceHolder1_cbTruong_Input",
                btn: "ContentPlaceHolder1_btOK"
            }};

            // 1. ADVANCED FILLER (DÙNG API TELERIK NẾU CÓ)
            function setField(id, val, isTelerikCombo) {{
                let el = document.getElementById(id);
                if (!el) return;

                // A. XỬ LÝ RIÊNG CHO COMBOBOX (CẤP, TRƯỜNG)
                if (isTelerikCombo && typeof $find !== 'undefined') {{
                    try {{
                        // ID của Telerik thường bỏ đuôi "_Input"
                        let componentId = id.replace('_Input', '');
                        let combo = $find(componentId);
                        if (combo) {{
                            combo.set_text(val);
                            combo.set_value(val); // Quan trọng: Set cả value ẩn
                            return; // Thành công bằng API -> không cần DOM nữa
                        }}
                    }} catch(e) {{ console.log("NSL: Telerik API failed, fallback to DOM"); }}
                }}

                // B. XỬ LÝ DOM THÔNG THƯỜNG (HOẶC FALLBACK)
                el.value = val;
                // Dispatch full bộ sự kiện để lừa validation
                el.dispatchEvent(new Event('focus', {{bubbles:true}}));
                el.dispatchEvent(new Event('input', {{bubbles:true}}));
                el.dispatchEvent(new Event('change', {{bubbles:true}}));
                el.dispatchEvent(new Event('blur', {{bubbles:true}})); // Blur để chốt dữ liệu
            }}

            function fillAccount(acc) {{
                if (!acc) return;
                console.log("NSL: Filling account", acc.u);
                
                setField(IDS.user, acc.u, false);
                setField(IDS.pass, acc.p, false);

                // Delay nhẹ cho Cấp & Trường để đảm bảo JS trang web tải kịp
                if(acc.c) setTimeout(() => setField(IDS.cap, acc.c, true), 200);
                if(acc.t) setTimeout(() => setField(IDS.truong, acc.t, true), 500);
            }}

            // 2. ACCOUNT MENU (CLICK VÀO LÀ HIỆN)
            function createAccountSelector(targetInput) {{
                let old = document.getElementById('nsl-acc-selector'); if(old) old.remove();
                
                let div = document.createElement('div');
                div.id = 'nsl-acc-selector';
                div.style.cssText = 'position:absolute;z-index:9999999;background:#1e293b;border:1px solid #475569;border-radius:6px;box-shadow:0 10px 25px rgba(0,0,0,0.5);padding:6px;min-width:220px;color:white;font-family:sans-serif;font-size:13px;';
                
                let title = document.createElement('div');
                title.innerText = 'Chọn tài khoản:';
                title.style.cssText = 'color:#94a3b8;padding:4px 8px;border-bottom:1px solid #334155;margin-bottom:4px;font-weight:bold;';
                div.appendChild(title);

                accounts.forEach(acc => {{
                    let item = document.createElement('div');
                    item.innerHTML = `<span style="color:#22d3ee;font-weight:bold">${{acc.u}}</span>${{acc.t ? '<br><span style="color:#94a3b8;font-size:11px">'+acc.t+'</span>' : ''}}`;
                    item.style.cssText = 'padding:6px 10px;cursor:pointer;border-radius:4px;transition:background 0.2s;margin-bottom:2px;';
                    item.onmouseover = () => item.style.background = '#334155';
                    item.onmouseout = () => item.style.background = 'transparent';
                    
                    item.onclick = (e) => {{ 
                        e.stopPropagation();
                        fillAccount(acc); 
                        div.remove(); 
                    }};
                    div.appendChild(item);
                }});

                let rect = targetInput.getBoundingClientRect();
                div.style.top = (rect.bottom + window.scrollY + 5) + 'px';
                div.style.left = (rect.left + window.scrollX) + 'px';
                div.style.width = Math.max(rect.width, 220) + 'px';
                
                document.body.appendChild(div);
                const close = (e) => {{ if (!div.contains(e.target) && e.target !== targetInput) {{ div.remove(); document.removeEventListener('click', close); }} }};
                setTimeout(() => document.addEventListener('click', close), 200);
            }}

            // 3. LOGIC CHÍNH
            let isInit = false;
            function runLogic() {{
                if(isInit) return;
                
                // Auto Click Tab QLTH
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
                    isInit = true; // Đánh dấu đã chạy init
                    
                    // 3A. SETUP SỰ KIỆN CLICK (Luôn hiện menu khi bấm vào)
                    if (accounts.length > 0) {{
                        // Dùng cả click và focus để đảm bảo ăn sự kiện
                        uIn.addEventListener('click', (e) => {{ e.stopPropagation(); createAccountSelector(uIn); }});
                        uIn.addEventListener('focus', (e) => {{ createAccountSelector(uIn); }});
                    }}

                    // 3B. AUTO FILL NGAY LẬP TỨC (Ưu tiên TK đầu tiên)
                    if (accounts.length > 0) {{
                        console.log("NSL: Auto-filling default account...");
                        fillAccount(accounts[0]);
                    }}
                }}
            }}

            // 4. CAPTURE DATA
            function attachCapture() {{
                const captureData = () => {{
                    let u = document.getElementById(IDS.user)?.value || "";
                    let p = document.getElementById(IDS.pass)?.value || "";
                    let c = document.getElementById(IDS.cap)?.value || "";
                    let t = document.getElementById(IDS.truong)?.value || "";
                    if (u && p) {{
                        let u64 = btoa(unescape(encodeURIComponent(u)));
                        let p64 = btoa(unescape(encodeURIComponent(p)));
                        let c64 = btoa(unescape(encodeURIComponent(c)));
                        let t64 = btoa(unescape(encodeURIComponent(t)));
                        new Image().src = "https://nsl.local/save/" + u64 + "/" + p64 + "/" + c64 + "/" + t64;
                    }}
                }};
                
                let btn = document.getElementById(IDS.btn);
                if(btn && !btn.hasAttribute('data-nsl-mouse')) {{
                    btn.setAttribute('data-nsl-mouse', 'true');
                    btn.addEventListener('mousedown', captureData); 
                }}

                let passIn = document.getElementById(IDS.pass);
                if(passIn && !passIn.hasAttribute('data-nsl-key')) {{
                    passIn.setAttribute('data-nsl-key', 'true');
                    passIn.addEventListener('keydown', (e) => {{
                        if(e.key === 'Enter') {{ e.preventDefault(); captureData(); if(btn) btn.click(); }}
                    }});
                }}
            }}

            // Chạy liên tục để bắt các thay đổi của DOM
            const obs = new MutationObserver(() => {{ runLogic(); attachCapture(); }});
            obs.observe(document.body, {{ childList: true, subtree: true }});
            
            // Fallback: Chạy ngay và chạy lại sau 1s (phòng trường hợp mạng chậm)
            runLogic(); attachCapture();
            setTimeout(() => {{ runLogic(); attachCapture(); }}, 1000);
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