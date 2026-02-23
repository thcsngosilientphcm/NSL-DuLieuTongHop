// src/menus/quanlytruonghoc.js
// Logic menu: Quản lý trường học (hcm.quanlytruonghoc.edu.vn)
//
// Menu QLTH có các submenu:
// - Điểm số: Nhập điểm, Nhập nhận xét môn học
// - Lịch báo giảng: Tổ trưởng nhập PPCT, Khai báo LBG
// - Sổ đầu bài: Nhập SDB, GVBM ký duyệt, GVCN ký duyệt

import { openBrowserView } from '../core/browser.js';

export function initQuanLyTruongHoc() {
    // Mở hệ thống QLTH
    const btnQlth = document.getElementById('btn-qlth');
    if (btnQlth) {
        btnQlth.addEventListener('click', function () {
            // Toggle submenu
            if (window.toggleMenu) window.toggleMenu('menu-qlth', this);
            // Mở browser tới trang chính
            openBrowserView(this.dataset.url, this.dataset.name);
        });
    }

    // Submenu items của QLTH
    document.querySelectorAll('#menu-qlth .menu-link[data-nav]').forEach(el => {
        el.addEventListener('click', function () {
            openBrowserView(this.dataset.nav, 'Quản lý trường học');
        });
    });
}
