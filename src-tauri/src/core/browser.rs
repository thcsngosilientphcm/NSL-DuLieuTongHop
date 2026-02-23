// src-tauri/src/browser.rs
// Logic chung xử lý embedded browser: helpers, monitor

use std::time::Duration;

use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde_json::Value;
use tauri::{AppHandle, Emitter};

use super::database::{load_store, perform_save_account, url_decode, AccountDTO, SECRET_KEY};

/// Lấy danh sách accounts dạng JSON cho domain hiện tại
pub fn get_accounts_json_for_domain(app: &AppHandle, raw_url: &str) -> String {
    let clean_domain = raw_url
        .replace("https://", "")
        .replace("http://", "")
        .split('/')
        .next()
        .unwrap_or("")
        .to_string();
    let store = load_store(app);
    let mc = new_magic_crypt!(SECRET_KEY, 256);
    let mut accounts_vec = Vec::new();
    if let Some(list) = store.accounts.get(&clean_domain) {
        for acc in list {
            let pass_plain = mc.decrypt_base64_to_string(&acc.pass).unwrap_or_default();
            accounts_vec.push(AccountDTO {
                id: acc.user.clone(),
                domain: clean_domain.clone(),
                website: raw_url.to_string(),
                username: acc.user.clone(),
                password: pass_plain,
            });
        }
    }
    serde_json::to_string(&accounts_vec).unwrap_or_else(|_| "[]".to_string())
}

/// Theo dõi URL hash của embedded browser để bắt lệnh save/trigger
pub fn setup_browser_monitor(view: tauri::Webview, app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_processed_hash = String::new();
        let mut last_user = String::new();
        let mut last_pass = String::new();
        loop {
            // Kiểm tra webview còn sống không
            if view.url().is_err() {
                break;
            }

            if let Ok(current_url) = view.url() {
                if let Some(fragment) = current_url.fragment() {
                    if fragment != last_processed_hash {
                        last_processed_hash = fragment.to_string();
                        // Xử lý lệnh SAVE
                        if fragment.starts_with("NSL_CMD_SAVE|") {
                            let parts: Vec<&str> = fragment.split('|').collect();
                            if parts.len() >= 3 {
                                let json_str = url_decode(parts[2]);
                                if let Ok(val) = serde_json::from_str::<Value>(&json_str) {
                                    let u = val["user"].as_str().unwrap_or("").to_string();
                                    let p = val["pass"].as_str().unwrap_or("").to_string();
                                    let _ =
                                        perform_save_account(&app, current_url.to_string(), u, p);
                                    let _ = app.emit("refresh-accounts", ());
                                }
                            }
                            let _ = view.eval("history.replaceState(null, null, ' ');");
                        } else if fragment.starts_with("NSL_DATA|") {
                            let parts: Vec<&str> = fragment.split('|').collect();
                            if parts.len() >= 3 {
                                let json_str = url_decode(parts[2]);
                                if let Ok(val) = serde_json::from_str::<Value>(&json_str) {
                                    last_user = val["user"].as_str().unwrap_or("").to_string();
                                    last_pass = val["pass"].as_str().unwrap_or("").to_string();
                                }
                            }
                        } else if fragment.starts_with("NSL_TRIGGER|")
                            || fragment.starts_with("NSL_TRIGGER_DATA|")
                        {
                            if fragment.starts_with("NSL_TRIGGER_DATA|") {
                                let parts: Vec<&str> = fragment.split('|').collect();
                                if parts.len() >= 3 {
                                    let json_str = url_decode(parts[2]);
                                    if let Ok(val) = serde_json::from_str::<Value>(&json_str) {
                                        last_user = val["user"].as_str().unwrap_or("").to_string();
                                        last_pass = val["pass"].as_str().unwrap_or("").to_string();
                                    }
                                }
                            }

                            // Người dùng bấm đăng nhập, kiểm tra tài khoản
                            if !last_user.is_empty() && !last_pass.is_empty() {
                                let status = crate::core::database::check_account_status(
                                    &app,
                                    current_url.as_str(),
                                    &last_user,
                                    &last_pass,
                                );

                                let u = last_user.clone();
                                let p = last_pass.clone();
                                let view_clone = view.clone();

                                match status {
                                    crate::core::database::AccountStatus::New => {
                                        let js = format!("if(typeof window.nslShowSavePrompt === 'function') {{ window.nslShowSavePrompt('{}', '{}', false); }}", u, p);
                                        let _ = view_clone.eval(&js);
                                    }
                                    crate::core::database::AccountStatus::UpdateRequired => {
                                        let js = format!("if(typeof window.nslShowSavePrompt === 'function') {{ window.nslShowSavePrompt('{}', '{}', true); }}", u, p);
                                        let _ = view_clone.eval(&js);
                                    }
                                    crate::core::database::AccountStatus::NoChange => {
                                        // Mật khẩu đúng, không làm gì cả
                                    }
                                }
                            }
                            let _ = view.eval("history.replaceState(null, null, ' ');");
                        } else if fragment.starts_with("NSL_REQ_ACCOUNTS|") {
                            let accounts_json =
                                get_accounts_json_for_domain(&app, current_url.as_str());
                            let js_update = format!("if(typeof window.__NSL_UPDATE_ACCOUNTS__ === 'function') {{ window.__NSL_UPDATE_ACCOUNTS__({}); }}", accounts_json);
                            let _ = view.eval(&js_update);
                            let _ = view.eval("history.replaceState(null, null, ' ');");
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}
