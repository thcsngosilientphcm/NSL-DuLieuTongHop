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

// --- HELPER FUNCTIONS (Giữ nguyên logic cũ) ---
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

// --- INJECTOR V21: TELERIK CORE INTEGRATION ---
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
        (function() {{
            console.log("🔥 NSL V21: Telerik Deep Integration Started");
            const accounts = {}; 
            const IDS = {{
                user: "ContentPlaceHolder1_tbU",
                pass: "ContentPlaceHolder1_tbP",
                cap: "ctl00_ContentPlaceHolder1_cbCapHoc", // Bỏ đuôi _Input để tìm component
                truong: "ctl00_ContentPlaceHolder1_cbTruong", // Bỏ đuôi _Input
                btn: "ContentPlaceHolder1_btOK"
            }};

            // 1. HÀM XỬ LÝ TELERIK COMBOBOX (Quan trọng nhất)
            function setTelerikCombo(componentId, textToSelect) {{
                if (!textToSelect) return;
                
                // Cố gắng tìm component qua API Telerik
                if (typeof $find !== 'undefined') {{
                    var combo = $find(componentId);
                    if (combo) {{
                        console.log("Found Telerik Combo:", componentId);
                        // Cách 1: Tìm item chính xác
                        var item = combo.findItemByText(textToSelect);
                        if (item) {{
                            item.select();
                            console.log("Selected item:", textToSelect);
                        }} else {{
                            // Cách 2: Set text và ép validate nếu không tìm thấy item (ít dùng)
                            combo.set_text(textToSelect);
                        }}
                        return; // Done qua API
                    }}
                }}

                // Fallback: Nếu API chưa load kịp, dùng DOM thuần nhưng kích hoạt sự kiện
                // Lưu ý: ID của input DOM thường có thêm "_Input"
                var el = document.getElementById(componentId + "_Input");
                if (el) {{
                    el.value = textToSelect;
                    el.dispatchEvent(new Event('focus'));
                    el.dispatchEvent(new Event('input'));
                    el.dispatchEvent(new Event('blur')); // Blur cực quan trọng để Telerik nhận diện
                }}
            }}

            // 2. HÀM ĐIỀN INPUT THƯỜNG (User/Pass)
            function setInput(id, val) {{
                var el = document.getElementById(id);
                if (el) {{
                    el.value = val;
                    // Trigger đủ bộ sự kiện để đánh lừa ASP.NET Validator
                    el.dispatchEvent(new Event('focus', {{bubbles:true}}));
                    el.dispatchEvent(new Event('keydown', {{bubbles:true}}));
                    el.dispatchEvent(new Event('input', {{bubbles:true}}));
                    el.dispatchEvent(new Event('change', {{bubbles:true}}));
                    el.dispatchEvent(new Event('blur', {{bubbles:true}}));
                }}
            }}

            // 3. QUY TRÌNH AUTO-FILL THÔNG MINH
            function fillAccount(acc) {{
                if (!acc) return;
                console.log("Filling account:", acc.u);

                // B1. Điền User & Pass
                setInput(IDS.user, acc.u);
                setInput(IDS.pass, acc.p);

                // B2. Điền Cấp học (Nếu có)
                if (acc.c) {{
                    // Delay nhẹ để đảm bảo JS trang web đã sẵn sàng
                    setTimeout(() => {{
                        setTelerikCombo(IDS.cap, acc.c);
                        
                        // B3. Điền Trường (Phải đợi Cấp học load xong mới điền được Trường)
                        // Tăng delay cho Trường lên 1 giây để đợi AJAX tải danh sách trường
                        if (acc.t) {{
                            setTimeout(() => {{
                                setTelerikCombo(IDS.truong, acc.t);
                            }}, 800); 
                        }}
                    }}, 200);
                }}
            }}

            // 4. MENU UI (Xử lý triệt để vụ click/ẩn hiện)
            function showMenu(targetInput) {{
                // Xóa menu cũ nếu có
                let old = document.getElementById('nsl-menu-overlay');
                if (old) old.remove();

                // Tạo Menu Wrapper
                let div = document.createElement('div');
                div.id = 'nsl-menu-overlay';
                div.style.cssText = 'position:absolute;z-index:9999999;background:#1e293b;border:1px solid #475569;border-radius:6px;box-shadow:0 10px 25px rgba(0,0,0,0.5);padding:6px;min-width:260px;color:white;font-family:Segoe UI, sans-serif;font-size:13px;';
                
                div.innerHTML = '<div style="color:#94a3b8;padding:4px 8px;border-bottom:1px solid #334155;margin-bottom:4px;font-weight:bold;font-size:12px;text-transform:uppercase">Chọn tài khoản lưu sẵn:</div>';

                accounts.forEach(acc => {{
                    let item = document.createElement('div');
                    item.innerHTML = `<div style="color:#22d3ee;font-weight:700;font-size:14px">${{acc.u}}</div><div style="color:#cbd5e1;font-size:11px;margin-top:2px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:240px;">${{acc.t || 'Chưa chọn trường'}}</div>`;
                    item.style.cssText = 'padding:8px 10px;cursor:pointer;border-radius:4px;margin-bottom:2px;transition:background 0.1s;';
                    
                    item.onmouseover = () => item.style.background = '#334155';
                    item.onmouseout = () => item.style.background = 'transparent';
                    
                    item.onmousedown = (e) => {{ // Dùng mousedown để bắt sự kiện trước blur input
                        e.preventDefault(); // Ngăn focus input bị mất
                        e.stopPropagation();
                        fillAccount(acc);
                        div.remove(); // Xóa ngay sau khi chọn
                    }};
                    div.appendChild(item);
                }});

                // Định vị
                let rect = targetInput.getBoundingClientRect();
                div.style.top = (rect.bottom + window.scrollY + 4) + 'px';
                div.style.left = (rect.left + window.scrollX) + 'px';
                
                document.body.appendChild(div);

                // Xử lý click ra ngoài để đóng
                setTimeout(() => {{
                    const closeHandler = (e) => {{
                        if (!div.contains(e.target) && e.target !== targetInput) {{
                            div.remove();
                            document.removeEventListener('click', closeHandler);
                        }}
                    }};
                    document.addEventListener('click', closeHandler);
                }}, 100);
            }}

            // 5. MAIN INIT LOGIC
            let hasAutoFilled = false;
            
            function init() {{
                // A. Auto Click Tab "Tài khoản QLTH" (Chỉ chạy 1 lần nếu cần)
                let tab = document.querySelector('.rtsTxt'); 
                // Logic tìm tab cụ thể nếu cần...

                let uIn = document.getElementById(IDS.user);
                if (uIn) {{
                    // B. Gắn sự kiện Click vào ô User
                    // Xóa listener cũ (thực ra code chạy lại sẽ tạo scope mới nhưng cứ gắn cờ cho chắc)
                    if (!uIn.dataset.nslHooked) {{
                        uIn.dataset.nslHooked = "true";
                        uIn.addEventListener('click', (e) => {{
                            if (accounts.length > 0) {{
                                e.stopPropagation();
                                showMenu(uIn);
                            }}
                        }});
                    }}

                    // C. Auto Fill lần đầu (Ưu tiên)
                    if (accounts.length > 0 && !hasAutoFilled && !uIn.value) {{
                        console.log("NSL: Auto-filling default...");
                        fillAccount(accounts[0]);
                        hasAutoFilled = true;
                    }}
                }}
            }}

            // 6. SAVING HOOK (Để lưu tài khoản mới)
            function attachSaveHook() {{
                const btn = document.getElementById(IDS.btn);
                if (btn && !btn.dataset.nslSave) {{
                    btn.dataset.nslSave = "true";
                    btn.addEventListener('mousedown', () => {{
                        let u = document.getElementById(IDS.user)?.value || "";
                        let p = document.getElementById(IDS.pass)?.value || "";
                        
                        // Lấy giá trị combo thì phức tạp hơn xíu
                        let c = "", t = "";
                        try {{
                             if(typeof $find !== 'undefined') {{
                                c = $find(IDS.cap)?.get_text() || "";
                                t = $find(IDS.truong)?.get_text() || "";
                             }}
                        }} catch(e) {{}}

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

            // Chạy vòng lặp kiểm tra (an toàn hơn MutationObserver trên các trang Telerik cũ)
            setInterval(() => {{
                init();
                attachSaveHook();
            }}, 1000);

            // Chạy ngay lần đầu
            setTimeout(init, 500);

        }})();
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