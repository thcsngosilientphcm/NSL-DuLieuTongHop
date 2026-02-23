// src/menus/csdlnganh.js
// Logic menu: CSDL Ngành (truong.hcm.edu.vn)
//
// Menu CSDL Ngành có submenu:
// - Ký học bạ

import { openBrowserView } from '../core/browser.js';

export function initCSDLNganh() {
    // Mở hệ thống CSDL Ngành
    const btnCsdl = document.getElementById('btn-csdl');
    if (btnCsdl) {
        btnCsdl.addEventListener('click', function () {
            // Toggle submenu
            if (window.toggleMenu) window.toggleMenu('menu-csdl', this);
            // Mở browser tới trang chính
            openBrowserView(this.dataset.url, this.dataset.name);
        });
    }

    // Submenu items của CSDL
    document.querySelectorAll('#menu-csdl .menu-link[data-nav]').forEach(el => {
        el.addEventListener('click', function () {
            openBrowserView(this.dataset.nav, 'CSDL Ngành');
        });
    });
}
