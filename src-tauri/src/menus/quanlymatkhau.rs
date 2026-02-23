// src-tauri/src/commands/accounts.rs
// Commands liên quan đến quản lý tài khoản

use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use tauri::{AppHandle, Manager};

use crate::core::browser::get_accounts_json_for_domain;
use crate::core::database::{
    get_all_accounts_impl, load_store, perform_save_account, save_store, AccountDTO, SECRET_KEY,
};

#[tauri::command]
pub fn get_all_accounts(app: AppHandle) -> Vec<AccountDTO> {
    get_all_accounts_impl(&app)
}

#[tauri::command]
pub fn save_account(
    app: AppHandle,
    domain: String,
    user: String,
    pass: String,
) -> Result<String, String> {
    perform_save_account(&app, domain, user, pass)
}

#[tauri::command]
pub fn delete_account(app: AppHandle, domain: String, username: String) -> Result<String, String> {
    let clean = domain
        .replace("https://", "")
        .replace("http://", "")
        .split('/')
        .next()
        .unwrap_or(&domain)
        .to_string();
    let mut store = load_store(&app);
    if let Some(list) = store.accounts.get_mut(&clean) {
        list.retain(|a| a.user != username);
        save_store(&app, &store)?;
        return Ok("OK".to_string());
    }
    Err("ERR".to_string())
}

#[tauri::command]
pub fn get_full_account_details(
    app: AppHandle,
    domain: String,
    username: String,
) -> Result<(String, String), String> {
    let clean = domain
        .replace("https://", "")
        .replace("http://", "")
        .split('/')
        .next()
        .unwrap_or(&domain)
        .to_string();
    let store = load_store(&app);
    let mc = new_magic_crypt!(SECRET_KEY, 256);
    if let Some(list) = store.accounts.get(&clean) {
        if let Some(acc) = list.iter().find(|a| a.user == username) {
            let pass = mc.decrypt_base64_to_string(&acc.pass).unwrap_or_default();
            return Ok((acc.user.clone(), pass));
        }
    }
    Err("Not found".to_string())
}

#[tauri::command]
pub fn refresh_autofill_data(app: AppHandle, url: String) {
    if url.is_empty() {
        return;
    }
    if let Some(view) = app.get_webview("browser_view") {
        let accounts_json = get_accounts_json_for_domain(&app, &url);
        let js = format!("if(typeof window.__NSL_UPDATE_ACCOUNTS__ === 'function') {{ window.__NSL_UPDATE_ACCOUNTS__({}); }}", accounts_json);
        let _ = view.eval(&js);
    }
}
