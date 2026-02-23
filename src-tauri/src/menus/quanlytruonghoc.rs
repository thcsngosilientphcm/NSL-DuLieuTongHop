// src-tauri/src/scripts/quanlytruonghoc.rs
// JS injection riêng cho hệ thống Quản Lý Trường Học (hcm.quanlytruonghoc.edu.vn)
//
// Các selector đặc trưng:
// - Input username: #ContentPlaceHolder1_tbU hoặc input[name*="$tbU"]
// - Input password: #ContentPlaceHolder1_tbP hoặc input[name*="$tbP"]
// - Nút đăng nhập: #ContentPlaceHolder1_btOK
// - Postback: WebForm_DoPostBackWithOptions

/// Script bổ sung riêng cho trang quanlytruonghoc
/// Hiện tại logic đã nằm trong autofill.rs, file này dùng để mở rộng sau
/// Ví dụ: tự động điền PPCT, tự nhập điểm, xử lý sổ đầu bài...
#[allow(dead_code)]
pub fn get_qlth_extra_script() -> String {
    r##"
        (function(){
            // === QUẢN LÝ TRƯỜNG HỌC - SCRIPT MỞ RỘNG ===
            // Chỗ này sẽ thêm logic xử lý đặc biệt cho QLTH
            // Ví dụ:
            // - Tự động nhập điểm hàng loạt
            // - Tự động khai báo Lịch Báo Giảng
            // - Hỗ trợ nhập Sổ Đầu Bài nhanh
            // - Ký duyệt tự động
            
            console.log('[NSL] Quản lý trường học module loaded');
        })();
    "##
    .to_string()
}
