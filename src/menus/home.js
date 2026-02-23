// src/menus/home.js
// Logic menu: Trang chủ

import { hideAllViews } from '../core/browser.js';

export function initHome() {
    document.getElementById('btn-home')?.addEventListener('click', () => {
        hideAllViews();
        document.getElementById('view-home').classList.remove('hidden');
        document.getElementById('page-title').innerText = 'Trang chủ';
    });
}
