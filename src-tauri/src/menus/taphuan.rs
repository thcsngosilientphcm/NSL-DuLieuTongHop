// src-tauri/src/menus/taphuan.rs
// JS injection riêng cho hệ thống Tập Huấn Cơ Sở (taphuan.csdl.edu.vn)
//
// Các selector đặc trưng:
// - Input username: input[name="lname"] hoặc input[placeholder="Tài khoản"]
// - Input password: input[name="pass"] hoặc input[type="password"]
// - Nút đăng nhập: .vt-login-form__login-button

/// Script bổ sung riêng cho trang Tập Huấn Cơ Sở
/// Hiện tại logic autofill đã nằm trong autofill.rs, file này dùng để mở rộng sau
#[allow(dead_code)]
pub fn get_taphuan_extra_script() -> String {
    r##"
        (function(){
            // === TẬP HUẤN CƠ SỞ - SCRIPT MỞ RỘNG ===
            // Chỗ này sẽ thêm logic xử lý đặc biệt cho Tập Huấn
            
            console.log('[NSL] Tập Huấn module loaded');
        })();
    "##
    .to_string()
}
