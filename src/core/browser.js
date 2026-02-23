// src/core/browser.js
// Logic embedded browser: bounds, open, hide

const invoke = window.__TAURI__?.core?.invoke;

let currentMainUrl = "";

export function getCurrentUrl() { return currentMainUrl; }
export function setCurrentUrl(url) { currentMainUrl = url; }

export function updateBrowserBounds() {
    const mount = document.getElementById('browser-mount-point');
    if (!mount || mount.classList.contains('hidden')) return;

    const rect = mount.getBoundingClientRect();

    if (invoke) {
        invoke('update_embedded_browser_bounds', {
            x: Math.round(rect.x),
            y: Math.round(rect.y),
            width: Math.round(rect.width),
            height: Math.round(rect.height)
        }).catch(console.error);
    }
}

export function hideAllViews() {
    ['view-home', 'view-passwords'].forEach(id => {
        const el = document.getElementById(id);
        if (el) el.classList.add('hidden');
    });

    // Ẩn browser mount point và báo Rust ẩn Webview đi
    const mount = document.getElementById('browser-mount-point');
    if (mount) mount.classList.add('hidden');

    if (invoke) invoke('hide_embedded_browser').catch(() => { });
}

export function openBrowserView(url, name) {
    // 1. Ẩn các view khác
    ['view-home', 'view-passwords'].forEach(id => {
        const el = document.getElementById(id);
        if (el) el.classList.add('hidden');
    });

    // 2. Hiện mount point
    const mount = document.getElementById('browser-mount-point');
    mount.classList.remove('hidden');
    document.getElementById('page-title').innerText = name || 'Hệ thống';

    currentMainUrl = url;

    // 3. Tính toán và gọi Rust
    const rect = mount.getBoundingClientRect();
    if (invoke) {
        invoke('open_embedded_browser', {
            url: url,
            x: Math.round(rect.x),
            y: Math.round(rect.y),
            width: Math.round(rect.width),
            height: Math.round(rect.height)
        });
    }
}

export function initBrowser() {
    // Setup ResizeObserver
    const mount = document.getElementById('browser-mount-point');
    if (mount) {
        const resizeObserver = new ResizeObserver(() => {
            requestAnimationFrame(() => updateBrowserBounds());
        });
        resizeObserver.observe(mount);
    }
    // Lưu ý: click handlers cho menu đã được chuyển vào từng module riêng
    // (menus/quanlytruonghoc.js, menus/csdlnganh.js, v.v.)
}

