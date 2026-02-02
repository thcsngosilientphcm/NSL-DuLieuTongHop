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

// --- COMMANDS ---
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

// --- INJECTOR V24: UI CLICK SIMULATION ---
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
            items.push(format!(r#"{{"u":"{}","p":"{}","c":"{}","t":"{}"}}"#, acc.user, p_dec, acc.cap, acc.truong));
        }
        accounts_json = format!("[{}]", items.join(","));
    }

    let init_script = format!(r#"
        window.addEventListener('DOMContentLoaded', () => {{
            const accounts = {}; 
            const IDS = {{
                user: "ContentPlaceHolder1_tbU",
                pass: "ContentPlaceHolder1_tbP",
                cap: "ctl00_ContentPlaceHolder1_cbCapHoc",
                truong: "ctl00_ContentPlaceHolder1_cbTruong",
                btn: "ContentPlaceHolder1_btOK"
            }};

            // HÀM QUAN TRỌNG: MÔ PHỎNG CLICK CHỌN ITEM ĐỂ TRIGGER TELERIK
            function forceSelectTelerik(comboId, textValue, callback) {{
                if (typeof $find === 'undefined' || !textValue) return;
                const combo = $find(comboId);
                if (!combo) return;

                console.log(">> Attempting Force Click for:", textValue);
                
                // 1. Mở danh sách sổ xuống
                combo.showDropDown();

                setTimeout(() => {{
                    // 2. Tìm Item DOM thực tế trong danh sách đã sổ
                    const item = combo.findItemByText(textValue);
                    if (item) {{
                        const itemElement = item.get_element();
                        if (itemElement) {{
                            // 3. Giả lập hành vi Click thật sự của chuột
                            itemElement.scrollIntoView();
                            item.click(); // Gọi method click của Telerik Item
                            console.log(">> Clicked item successfully");
                        }}
                    }}
                    // 4. Đóng danh sách
                    combo.hideDropDown();
                    if (callback) callback();
                }}, 300); // Chờ 300ms để dropdown render xong
            }}

            function smartFill(acc) {{
                if (!acc) return;
                // Điền User/Pass
                const u = document.getElementById(IDS.user);
                const p = document.getElementById(IDS.pass);
                if (u) u.value = acc.u;
                if (p) p.value = acc.p;

                // THỰC HIỆN CHỌN THEO LUỒNG: Cấp -> Trường
                forceSelectTelerik(IDS.cap, acc.c, () => {{
                    console.log(">> Waiting for School list postback...");
                    // Đợi 1.5 giây để ASP.NET thực hiện Postback và load lại Dropdown Trường
                    setTimeout(() => {{
                        forceSelectTelerik(IDS.truong, acc.t);
                    }}, 1500);
                }});
            }}

            function ensureTab() {{
                const spans = document.querySelectorAll('.rtsTxt');
                for (let s of spans) {{
                    if (s.innerText.trim() === "Tài khoản QLTH") {{
                        const link = s.closest('a.rtsLink');
                        if (link && !link.classList.contains('rtsSelected')) link.click();
                        break;
                    }}
                }}
            }}

            function showMenu() {{
                const uIn = document.getElementById(IDS.user); if (!uIn) return;
                let div = document.getElementById('nsl-menu'); if (div) div.remove();
                div = document.createElement('div');
                div.id = 'nsl-menu';
                div.style.cssText = 'position:absolute;z-index:999999;background:#1e293b;border:1px solid #475569;border-radius:6px;box-shadow:0 10px 25px rgba(0,0,0,0.5);padding:6px;min-width:250px;color:white;cursor:default;';
                div.innerHTML = '<div style="color:#94a3b8;padding:4px 8px;border-bottom:1px solid #334155;margin-bottom:4px;font-size:12px;">CHỌN TÀI KHOẢN:</div>';

                accounts.forEach(acc => {{
                    const item = document.createElement('div');
                    item.innerHTML = `<div style="color:#22d3ee;font-weight:bold">${{acc.u}}</div><div style="font-size:11px;color:#94a3b8">${{acc.t}}</div>`;
                    item.style.cssText = 'padding:8px;border-radius:4px;cursor:pointer;margin-bottom:2px;';
                    item.onmouseover = () => item.style.background = '#334155';
                    item.onmouseout = () => item.style.background = 'transparent';
                    item.onmousedown = (e) => {{ e.preventDefault(); e.stopPropagation(); smartFill(acc); div.remove(); }};
                    div.appendChild(item);
                }});

                const r = uIn.getBoundingClientRect();
                div.style.top = (r.bottom + window.scrollY + 5) + 'px';
                div.style.left = (r.left + window.scrollX) + 'px';
                document.body.appendChild(div);

                const close = (e) => {{ if (!div.contains(e.target) && e.target !== uIn) {{ div.remove(); document.removeEventListener('click', close); }} }};
                setTimeout(() => document.addEventListener('click', close), 100);
            }}

            let isFirst = true;
            setInterval(() => {{
                ensureTab();
                const uIn = document.getElementById(IDS.user);
                if (uIn && !uIn.dataset.hook) {{
                    uIn.dataset.hook = "true";
                    uIn.addEventListener('click', (e) => {{ e.stopPropagation(); showMenu(); }});
                    if (isFirst && accounts.length > 0 && !uIn.value) {{
                        setTimeout(() => smartFill(accounts[0]), 1000);
                        isFirst = false;
                    }}
                }}
            }}, 1000);
        }});
    "#, accounts_json);

    if let Some(win) = app.get_webview_window("embedded_browser") { let _ = win.close(); }
    let main = app.get_webview_window("main").unwrap();
    let size = main.inner_size().unwrap();
    let _ = WebviewWindowBuilder::new(&app, "embedded_browser", WebviewUrl::External(url.parse().unwrap()))
        .title("Browser").decorations(false).parent(&main).unwrap()
        .inner_size((size.width as f64) - 260.0, (size.height as f64) - 64.0).position(260.0, 64.0)
        .initialization_script(&init_script)
        .on_navigation(move |u: &Url| {{
            if u.as_str().starts_with("https://nsl.local/save/") {{
                let p: Vec<&str> = u.as_str().split('/').collect();
                if p.len() >= 8 {{
                    let decode = |idx: usize| String::from_utf8(general_purpose::STANDARD.decode(p[idx]).unwrap_or_default()).unwrap_or_default();
                    let _ = perform_save_account(&app, domain.clone(), decode(4), decode(5), decode(6), decode(7));
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
        .invoke_handler(tauri::generate_handler![save_account, get_all_accounts, delete_account, open_secure_window, navigate_webview, hide_embedded_view, update_webview_layout])
        .run(tauri::generate_context!())
        .expect("error");
}