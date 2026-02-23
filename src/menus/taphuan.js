// src/menus/taphuan.js
// Logic menu: Tập huấn: Bồi Dưỡng GVPT và CBQL (taphuan.csdl.edu.vn)

import { openBrowserView } from '../core/browser.js';

export function initTapHuan() {
    const btn = document.getElementById('btn-taphuan');
    if (btn) {
        btn.addEventListener('click', function () {
            openBrowserView('https://taphuan.csdl.edu.vn/', 'Tập huấn: Bồi Dưỡng GVPT và CBQL');
        });
    }
}
