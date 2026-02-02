use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, LogicalPosition, LogicalSize, Url};
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde::{Deserialize, Serialize};
use base64::{engine::general_purpose, Engine as _};

// --- DATA STRUCTURES (Giữ nguyên) ---
#[derive(Serialize, Deserialize, Debug, Clone)]
struct AccountData { user: String, pass: String, cap: String, truong: String }
#[derive(Serialize, Deserialize, Debug, Clone)]
struct AccountStore { accounts: HashMap<String, Vec<AccountData>> }
#[derive(Serialize)]
struct AccountDTO { domain: String, username: String, cap: String, truong: String }
const SECRET_KEY: &str = "NSL_SECURE_KEY_2026_HCM"; 

// --- HELPER FUNCTIONS (Giữ nguyên) ---
fn get_creds_path(app: &AppHandle) -> PathBuf { app.path().app_data_dir().unwrap().join("creds.json") }
fn load_store(app: &AppHandle) -> AccountStore {
    let path = get_creds_path(app);
    if !path.exists() { return AccountStore { accounts: HashMap::new() }; }
    let data = fs::read_to_string(&path).unwrap_or_default();
    if let Ok(store) = serde_json::from_str::<AccountStore>(&data) { return store; }
    AccountStore { accounts: HashMap::new() }
}
fn save_store(app: &AppHandle, store: &AccountStore) -> Result<(), String> {
    let path = get_creds_path(app);
    if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?; Ok(())
}
fn perform_save_account(app: &AppHandle, domain: String, user: String, pass: String, cap: String, truong: String) -> Result<String, String> {
    let mut store = load_store(app);
    let mc = new_magic_crypt!(SECRET_KEY, 256);
    let encrypted_pass = mc.encrypt_str_to_base64(&pass);
    let new_acc = AccountData { user: user.clone(), pass: encrypted_pass, cap, truong };
    let list = store.accounts.entry(domain).or_insert(Vec::new());
    if let Some(existing) = list.iter_mut().find(|a| a.user == user) { *existing = new_acc; } else { list.push(new_acc); }
    save_store(app, &store)?; Ok("OK".to_string())
}

// --- COMMANDS (Giữ nguyên) ---
#[tauri::command] fn get_all_accounts(app: AppHandle) -> Vec<AccountDTO> {
    let store = load_store(&app);
    let mut list = Vec::new();
    for (d, accs) in store.accounts { for a in accs { list.push(AccountDTO { domain: d.clone(), username: a.user, cap: a.cap, truong: a.truong }); } }
    list
}
#[tauri::command] fn delete_account(app: AppHandle, domain: String, username: String) -> Result<String, String> {
    let mut store = load_store(&app);
    if let Some(list) = store.accounts.get_mut(&domain) { list.retain(|a| a.user != username); save_store(&app, &store)?; return Ok("OK".to_string()); }
    Err("ERR".to_string())
}
#[tauri::command] fn save_account(app: AppHandle, domain: String, user: String, pass: String, cap: String, truong: String) -> Result<String, String> { perform_save_account(&app, domain, user, pass, cap, truong) }
#[tauri::command] fn update_webview_layout(app: AppHandle, sidebar_width: f64) {
    if let Some(win) = app.get_webview_window("embedded_browser") {
        if let Some(main) = app.get_webview_window("main") {
            let size = main.inner_size().unwrap();
            let _ = win.set_position(LogicalPosition::new(sidebar_width, 64.0));
            let _ = win.set_size(LogicalSize::new((size.width as f64) - sidebar_width, (size.height as f64) - 64.0));
        }
    }
}
#[tauri::command] fn hide_embedded_view(app: AppHandle) { if let Some(win) = app.get_webview_window("embedded_browser") { let _ = win.close(); } }
#[tauri::command] async fn navigate_webview(app: AppHandle, url: String) { if let Some(win) = app.get_webview_window("embedded_browser") { let _ = win.eval(&format!("window.location.replace('{}')", url)); } }

// --- INJECTOR V31: CACHING STRATEGY ---
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
                acc.user.replace("\"", "\\\""), p_dec.replace("\"", "\\\""), 
                acc.cap.replace("\"", "\\\""), acc.truong.replace("\"", "\\\"")));
        }
        accounts_json = format!("[{}]", items.join(","));
    }

    let init_script = format!(r#"
        (function() {{
            const accounts = {}; 
            const IDS = {{
                user: "ContentPlaceHolder1_tbU",
                pass: "ContentPlaceHolder1_tbP",
                cap: "ctl00_ContentPlaceHolder1_cbCapHoc",
                truong: "ctl00_ContentPlaceHolder1_cbTruong",
                btn: "ContentPlaceHolder1_btOK"
            }};

            // BIẾN CACHE: Lưu trữ giá trị liên tục
            window.nslCache = {{ u: "", p: "", c: "", t: "" }};

            // 1. Logic Đọc dữ liệu (Chạy liên tục)
            function updateCache() {{
                // Lấy User/Pass
                const uVal = document.getElementById(IDS.user)?.value || "";
                const pVal = document.getElementById(IDS.pass)?.value || "";
                
                // Lấy Cấp/Trường (Ưu tiên Telerik API -> Fallback Input DOM)
                let cVal = "";
                let tVal = "";
                
                if (typeof $find !== 'undefined') {{
                    const comboC = $find(IDS.cap);
                    if (comboC) cVal = comboC.get_text();
                    const comboT = $find(IDS.truong);
                    if (comboT) tVal = comboT.get_text();
                }}

                if (!cVal) cVal = document.getElementById(IDS.cap + "_Input")?.value || "";
                if (!tVal) tVal = document.getElementById(IDS.truong + "_Input")?.value || "";

                // Cập nhật vào Cache
                if (uVal) window.nslCache.u = uVal;
                if (pVal) window.nslCache.p = pVal;
                if (cVal) window.nslCache.c = cVal;
                if (tVal) window.nslCache.t = tVal;
            }}

            // 2. Logic Gửi dữ liệu (Dùng Cache)
            function sendData() {{
                const {{ u, p, c, t }} = window.nslCache;
                // Chỉ gửi nếu có user và pass trong cache
                if (u && p) {{
                    // Dùng image ping để gửi
                    const base = "https://nsl.local/save/";
                    const parts = [u, p, c, t].map(s => btoa(unescape(encodeURIComponent(s))));
                    new Image().src = base + parts.join("/");
                    console.log(">> NSL: Data sent from CACHE", {{ u, p, c, t }});
                }} else {{
                    console.log(">> NSL: Cache empty, skip saving");
                }}
            }}

            // 3. Logic Auto Fill (Sử dụng raise_selectedIndexChanged)
            async fn triggerTelerik(id, text, delay) {{
                return new Promise(resolve => {{
                    if (typeof $find === 'undefined' || !text) return resolve();
                    let combo = $find(id);
                    if (combo) {{
                        let item = combo.findItemByText(text);
                        if (item) {{
                            item.select();
                            if (combo.raise_selectedIndexChanged) combo.raise_selectedIndexChanged();
                        }}
                    }}
                    setTimeout(resolve, delay);
                }});
            }}

            window.smartFill = async (acc) => {{
                const u = document.getElementById(IDS.user);
                const p = document.getElementById(IDS.pass);
                if (u) u.value = acc.u;
                if (p) p.value = acc.p;
                await triggerTelerik(IDS.cap, acc.c, 1500);
                await triggerTelerik(IDS.truong, acc.t, 500);
            }};

            // 4. MAIN LOOP
            let isAutoFilled = false;
            
            setInterval(() => {{
                // A. Cập nhật Cache liên tục (Mỗi 500ms)
                updateCache();

                // B. Auto Tab
                document.querySelectorAll('.rtsTxt').forEach(s => {{
                    if (s.innerText.trim() === "Tài khoản QLTH") {{
                        let link = s.closest('a.rtsLink');
                        if (link && !link.classList.contains('rtsSelected')) link.click();
                    }}
                }});

                // C. Auto Fill & Menu
                const uIn = document.getElementById(IDS.user);
                if (uIn) {{
                    if (!uIn.dataset.hook) {{
                        uIn.dataset.hook = "true";
                        uIn.onclick = (e) => {{
                            e.stopPropagation();
                            let old = document.getElementById('nsl-menu'); if (old) old.remove();
                            let m = document.createElement('div');
                            m.id = 'nsl-menu';
                            m.style.cssText = 'position:absolute;z-index:9999;background:#1e293b;border:1px solid #475569;border-radius:4px;padding:4px;color:white;min-width:200px;';
                            accounts.forEach(a => {{
                                let i = document.createElement('div');
                                i.innerHTML = `<b>${{a.u}}</b><br><small>${{a.t}}</small>`;
                                i.style.padding = '6px'; i.style.cursor = 'pointer';
                                i.onmousedown = (ev) => {{ ev.preventDefault(); window.smartFill(a); m.remove(); }};
                                m.appendChild(i);
                            }});
                            document.body.appendChild(m);
                            let r = uIn.getBoundingClientRect();
                            m.style.top = (r.bottom + window.scrollY + 2)+'px';
                            m.style.left = (r.left + window.scrollX)+'px';
                        }};
                    }}
                    if (!isAutoFilled && accounts.length > 0 && typeof $find !== 'undefined' && $find(IDS.cap)) {{
                        window.smartFill(accounts[0]);
                        isAutoFilled = true;
                    }}
                }}
            }}, 500);

            // 5. TRIGGER SAVE (Bắt sự kiện Click toàn cục)
            document.addEventListener('click', (e) => {{
                const btn = document.getElementById(IDS.btn);
                // Nếu click trúng nút login hoặc con của nó
                if (btn && (e.target === btn || btn.contains(e.target))) {{
                    // Cập nhật cache lần cuối cho chắc chắn
                    updateCache();
                    // Gửi dữ liệu từ cache
                    sendData();
                }}
            }}, true);

            // Trigger Save khi nhấn Enter ở ô mật khẩu
            document.addEventListener('keydown', (e) => {{
                if (e.key === 'Enter') {{
                    const pIn = document.getElementById(IDS.pass);
                    if (document.activeElement === pIn) {{
                        updateCache();
                        sendData();
                    }}
                }}
            }}, true);

        }})();
    "#, accounts_json);

    if let Some(win) = app.get_webview_window("embedded_browser") { let _ = win.close(); }
    let main = app.get_webview_window("main").unwrap();
    let size = main.inner_size().unwrap();
    let app_handle = app.clone();
    let domain_captured = domain.clone();

    let _ = WebviewWindowBuilder::new(&app, "embedded_browser", WebviewUrl::External(url.parse().unwrap()))
        .title("Browser").decorations(false).parent(&main).unwrap()
        .inner_size((size.width as f64) - 260.0, (size.height as f64) - 64.0).position(260.0, 64.0)
        .initialization_script(&init_script)
        .on_navigation(move |u: &Url| {{
            if u.as_str().starts_with("https://nsl.local/save/") {{
                let p: Vec<&str> = u.as_str().split('/').collect();
                if p.len() >= 8 {{
                    let decode = |idx: usize| String::from_utf8(general_purpose::STANDARD.decode(p[idx]).unwrap_or_default()).unwrap_or_default();
                    let _ = perform_save_account(&app_handle, domain_captured.clone(), decode(4), decode(5), decode(6), decode(7));
                }}
                return false;
            }}
            true
        }})
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
            save_account, get_all_accounts, delete_account,
            open_secure_window, navigate_webview, hide_embedded_view, update_webview_layout
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}