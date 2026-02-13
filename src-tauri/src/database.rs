use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

// Cho phép bên ngoài truy cập khóa bí mật
pub const SECRET_KEY: &str = "NSL_SECURE_KEY_2026_HCM";

// --- Data Structures ---
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountData {
    pub user: String,
    pub pass: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountStore {
    pub accounts: HashMap<String, Vec<AccountData>>,
}

#[derive(Serialize)]
pub struct AccountDTO {
    pub id: String,
    pub domain: String,
    pub website: String,
    pub username: String,
    pub password: String,
}

#[derive(PartialEq)]
#[allow(dead_code)]
pub enum AccountStatus {
    New,            // Tài khoản chưa từng tồn tại
    UpdateRequired, // Tài khoản có tồn tại nhưng mật khẩu khác
    NoChange,       // Giống hệt
}

// --- Helpers ---
fn hex_char(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(10 + (b - b'a')),
        b'A'..=b'F' => Some(10 + (b - b'A')),
        _ => None,
    }
}

pub fn url_decode(input: &str) -> String {
    let mut out = String::new();
    let mut i = 0usize;
    let bytes = input.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_char(bytes[i + 1]), hex_char(bytes[i + 2])) {
                out.push(((h << 4) + l) as char);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

pub fn resolve_data_path(app: &AppHandle) -> PathBuf {
    let path_c = PathBuf::from("C:\\NSL_DATA");
    if !path_c.exists() {
        let _ = fs::create_dir_all(&path_c);
    }

    let mut final_path = if path_c.exists() {
        path_c
    } else {
        app.path().document_dir().unwrap_or(PathBuf::from("."))
    };
    final_path.push("creds.json");
    final_path
}

pub fn load_store(app: &AppHandle) -> AccountStore {
    let path = resolve_data_path(app);
    if !path.exists() {
        return AccountStore {
            accounts: HashMap::new(),
        };
    }
    let data = fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str::<AccountStore>(&data).unwrap_or(AccountStore {
        accounts: HashMap::new(),
    })
}

pub fn save_store(app: &AppHandle, store: &AccountStore) -> Result<(), String> {
    let path = resolve_data_path(app);
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&path, &json).map_err(|e| e.to_string())?;
    Ok(())
}

// --- Logic kiểm tra trạng thái tài khoản ---
#[allow(dead_code)]
pub fn check_account_status(
    app: &AppHandle,
    domain: &str,
    user: &str,
    pass: &str,
) -> AccountStatus {
    let clean_domain = domain
        .replace("https://", "")
        .replace("http://", "")
        .split('/')
        .next()
        .unwrap_or(domain)
        .to_string();
    let store = load_store(app);

    if let Some(list) = store.accounts.get(&clean_domain) {
        if let Some(acc) = list.iter().find(|a| a.user == user) {
            let mc = new_magic_crypt!(SECRET_KEY, 256);
            let stored_pass = mc.decrypt_base64_to_string(&acc.pass).unwrap_or_default();

            if stored_pass == pass {
                return AccountStatus::NoChange;
            } else {
                return AccountStatus::UpdateRequired;
            }
        }
    }
    AccountStatus::New
}

pub fn perform_save_account(
    app: &AppHandle,
    domain: String,
    user: String,
    pass: String,
) -> Result<String, String> {
    let clean_user = user.trim().to_string();
    let clean_pass = pass.trim().to_string();
    let mut store = load_store(app);
    let mc = new_magic_crypt!(SECRET_KEY, 256);
    let encrypted_pass = mc.encrypt_str_to_base64(&clean_pass);

    let clean_domain = domain
        .replace("https://", "")
        .replace("http://", "")
        .split('/')
        .next()
        .unwrap_or(&domain)
        .to_string();
    let list = store.accounts.entry(clean_domain).or_insert_with(Vec::new);

    if let Some(existing) = list.iter_mut().find(|a| a.user == clean_user) {
        existing.pass = encrypted_pass;
    } else {
        list.push(AccountData {
            user: clean_user,
            pass: encrypted_pass,
        });
    }
    save_store(app, &store)?;
    Ok("Saved".to_string())
}

pub fn get_all_accounts_impl(app: &AppHandle) -> Vec<AccountDTO> {
    let store = load_store(app);
    let mc = new_magic_crypt!(SECRET_KEY, 256);
    let mut list = Vec::new();
    for (domain_key, accs) in store.accounts {
        for a in accs {
            let decrypted = mc.decrypt_base64_to_string(&a.pass).unwrap_or_default();
            list.push(AccountDTO {
                id: format!("{}_{}", domain_key, a.user),
                domain: domain_key.clone(),
                website: format!("https://{}/", domain_key),
                username: a.user,
                password: decrypted,
            });
        }
    }
    list
}
