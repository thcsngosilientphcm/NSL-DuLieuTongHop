// src-tauri/src/commands/browser_cmds.rs
// Commands liên quan đến embedded browser

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Rect, WebviewBuilder, WebviewUrl};

use crate::core::browser::{get_accounts_json_for_domain, setup_browser_monitor};
use crate::menus::get_autofill_script;

#[tauri::command]
pub async fn open_embedded_browser(
    app: AppHandle,
    url: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let accounts_json = get_accounts_json_for_domain(&app, &url);
    let autofill_script = get_autofill_script();
    let safe_url = url.replace('\\', "\\\\").replace('\'', "\\'");

    // 1. Kiểm tra xem Webview con đã tồn tại chưa
    if let Some(view) = app.get_webview("browser_view") {
        // Cập nhật vị trí
        let _ = view.set_bounds(Rect {
            position: PhysicalPosition { x, y }.into(),
            size: PhysicalSize { width, height }.into(),
        });
        let _ = view.show();
        let _ = view.set_focus();

        // Navigate nếu URL khác (requestAccounts trong init script sẽ tự load data đúng domain)
        if let Ok(current) = view.url() {
            if current.to_string() != url {
                let _ = view.eval(&format!("window.location.href = '{}';", safe_url));
            } else {
                // Cùng URL → chỉ cần refresh accounts data
                let js_update = format!("if(typeof window.__NSL_UPDATE_ACCOUNTS__ === 'function') {{ window.__NSL_UPDATE_ACCOUNTS__({}); }}", accounts_json);
                let _ = view.eval(&js_update);
            }
        } else {
            let _ = view.eval(&format!("window.location.href = '{}';", safe_url));
        }
    } else {
        // 2. Tạo mới Webview con (Gắn vào window Main)
        println!("[NSL] Creating Embedded Browser View...");
        let main_win = app
            .get_webview_window("main")
            .ok_or("Main window not found")?;

        // Dùng Window::add_child() (API Tauri 2.10+)
        let builder =
            WebviewBuilder::new("browser_view", WebviewUrl::External(url.parse().unwrap()))
                .initialization_script(&autofill_script);

        let webview = main_win
            .as_ref()
            .window()
            .add_child(
                builder,
                PhysicalPosition { x, y },
                PhysicalSize { width, height },
            )
            .map_err(|e| format!("Failed to create webview: {}", e))?;

        // Setup logic lắng nghe login
        let app_clone = app.clone();
        setup_browser_monitor(webview, app_clone);
    }
    Ok(())
}

#[tauri::command]
pub fn update_embedded_browser_bounds(app: AppHandle, x: i32, y: i32, width: u32, height: u32) {
    if let Some(view) = app.get_webview("browser_view") {
        let _ = view.set_bounds(Rect {
            position: PhysicalPosition { x, y }.into(),
            size: PhysicalSize { width, height }.into(),
        });
    }
}

#[tauri::command]
pub fn hide_embedded_browser(app: AppHandle) {
    if let Some(view) = app.get_webview("browser_view") {
        let _ = view.hide();
    }
}
