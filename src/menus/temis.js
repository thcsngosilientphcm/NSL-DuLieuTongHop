// src/menus/temis.js
// Logic menu: Temis - Đánh giá chuẩn (temis.csdl.edu.vn)

import { openBrowserView } from '../core/browser.js';

export function initTemis() {
    const btn = document.getElementById('btn-temis');
    if (btn) {
        btn.addEventListener('click', function () {
            openBrowserView('https://temis.csdl.edu.vn/', 'Temis: Đánh giá chuẩn');
        });
    }
}
