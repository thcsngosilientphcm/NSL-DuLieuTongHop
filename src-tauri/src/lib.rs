use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, Listener};
use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AccountStore {
    accounts: HashMap<String, (String, String)>,
}

const SECRET_KEY: &str = "NSL_SECURE_KEY_2026_HCM"; 

fn get_creds_path(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap().join("creds.json")
}

// --- HÀM LƯU DỮ LIỆU (ĐÃ TỐI ƯU: KIỂM TRA TRƯỚC KHI LƯU) ---
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
    
    // --- BƯỚC KIỂM TRA THÔNG MINH ---
    if let Some((stored_user, stored_pass_enc)) = store.accounts.get(&domain) {
        // Nếu user giống nhau, thì mới kiểm tra tiếp password
        if stored_user == &user {
            // Giải mã password cũ để so sánh
            if let Ok(stored_pass_dec) = mc.decrypt_base64_to_string(stored_pass_enc) {
                if stored_pass_dec == pass {
                    println!(">> [SKIP] Dữ liệu không thay đổi. Bỏ qua ghi file.");
                    return Ok("Dữ liệu không đổi".to_string());
                }
            }
        }
    }

    // Nếu khác biệt, tiến hành mã hóa và lưu
    if !user.trim().is_empty() && !pass.trim().is_empty() {
        let encrypted_pass = mc.encrypt_str_to_base64(&pass);
        store.accounts.insert(domain.clone(), (user, encrypted_pass));
        
        let json = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())?;
        
        println!(">> [UPDATED] Đã cập nhật tài khoản mới cho: {}", domain);
        return Ok("Đã lưu thành công!".to_string());
    }
    
    Err("Dữ liệu rỗng".to_string())
}

#[tauri::command]
fn save_account(app: AppHandle, domain: String, user: String, pass: String) -> Result<String, String> {
    perform_save_account(&app, domain, user, pass)
}

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

    let init_script = format!(r#"
        window.addEventListener('DOMContentLoaded', () => {{
            console.log("🔥 NSL Smart Injector Active");

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
                const savedUser = "{}";
                const savedPass = "{}";
                if (!savedUser) return;

                let uInput = document.querySelector('input[name*="UserName"]') || document.querySelector('input[id*="user"]');
                let pInput = document.querySelector('input[name*="Password"]') || document.querySelector('input[id*="pass"]');

                if (uInput && pInput && !uInput.value) {{
                    uInput.value = savedUser;
                    pInput.value = savedPass;
                    uInput.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    pInput.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    
                    let cap = document.querySelector('input[name*="Captcha"]');
                    if(cap) cap.focus();
                }}
            }}

            function setupCapture() {{
                function sendToRust() {{
                    let uInput = document.querySelector('input[name*="UserName"]') || document.querySelector('input[id*="user"]');
                    let pInput = document.querySelector('input[name*="Password"]') || document.querySelector('input[id*="pass"]');
                    
                    if (uInput && pInput && uInput.value && pInput.value) {{
                        document.title = "NSL_SAVE:::" + uInput.value + ":::" + pInput.value;
                    }}
                }}

                document.addEventListener('keydown', (e) => {{ if (e.key === 'Enter') sendToRust(); }});
                document.addEventListener('click', (e) => {{
                    let target = e.target;
                    while (target && target !== document) {{
                        if (target.type === 'submit' || target.id.toLowerCase().includes('login') || target.innerText.toLowerCase().includes('đăng nhập')) {{
                            sendToRust(); break;
                        }}
                        target = target.parentElement;
                    }}
                }});
            }}

            setTimeout(autoClickTab, 500);
            setTimeout(autoClickTab, 1500);
            setTimeout(autoFill, 800);
            setupCapture();
        }});
    "#, username, password);

    let window_label = format!("win_{}", domain.replace(".", "_"));
    if let Some(win) = app.get_webview_window(&window_label) {
        let _ = win.close();
    }

    let window = WebviewWindowBuilder::new(&app, &window_label, WebviewUrl::External(url.parse().unwrap()))
        .title("Hệ thống NSL - Secure Browser")
        .inner_size(1200.0, 800.0)
        .initialization_script(&init_script)
        .build()
        .unwrap();

    let app_handle_clone = app.clone();
    let domain_clone = domain.clone();
    let window_clone = window.clone();

    window.on_window_event(move |event| {
        if let tauri::WindowEvent::TitleChanged(title) = event {
            if title.starts_with("NSL_SAVE:::") {
                let parts: Vec<&str> = title.split(":::").collect();
                if parts.len() >= 3 {
                    let user = parts[1].to_string();
                    let pass = parts[2].to_string();
                    let _ = perform_save_account(&app_handle_clone, domain_clone.clone(), user, pass);
                    let _ = window_clone.set_title("Hệ thống NSL - Đang đăng nhập...");
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![save_account, open_secure_window])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}