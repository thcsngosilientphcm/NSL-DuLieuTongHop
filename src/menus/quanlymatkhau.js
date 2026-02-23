// src/menus/quanlymatkhau.js
// Logic menu: Quản lý mật khẩu

const invoke = window.__TAURI__?.core?.invoke;
const listen = window.__TAURI__?.event?.listen;

import { openModal, showCustomConfirm } from '../core/modal.js';
import { hideAllViews, getCurrentUrl } from '../core/browser.js';

// Sync data tới embedded browser
async function syncDataToBrowser() {
    const url = getCurrentUrl();
    if (url && invoke) {
        await invoke('refresh_autofill_data', { url }).catch(() => { });
    }
}

// Tải danh sách mật khẩu
async function loadPasswordTable() {
    const tbody = document.getElementById('password-table-body');
    if (!tbody) return;
    tbody.innerHTML = '<tr><td colspan="4" class="text-center text-slate-500 py-4">Đang tải...</td></tr>';
    const accounts = await (invoke ? invoke('get_all_accounts') : Promise.resolve([]));
    tbody.innerHTML = '';
    if (!accounts || accounts.length === 0) {
        tbody.innerHTML = '<tr><td colspan="4" class="text-center text-slate-500 py-4">Chưa có dữ liệu</td></tr>';
        return;
    }
    accounts.forEach((acc, i) => {
        const tr = document.createElement('tr');
        tr.className = "hover:bg-white/5 border-b border-white/5";
        tr.innerHTML = `<td class="text-center p-3">${i + 1}</td><td class="p-3">${acc.domain}</td><td class="text-cyan-300 p-3">${acc.username}</td>
        <td class="flex justify-center gap-2 p-3">
            <button class="p-1.5 bg-slate-700 rounded" data-edit="${acc.domain}|${acc.username}" title="Chỉnh sửa">✏️</button>
            <button class="p-1.5 bg-slate-700 rounded" data-copy="${acc.domain}|${acc.username}" title="Sao chép mật khẩu">📋</button>
            <button class="p-1.5 bg-slate-700 rounded" data-delete="${acc.domain}|${acc.username}" title="Xóa">🗑️</button>
        </td>`;
        tbody.appendChild(tr);
    });
    tbody.querySelectorAll('button[data-edit]').forEach(b => b.addEventListener('click', async () => {
        const [d, u] = b.dataset.edit.split('|');
        const det = await invoke('get_full_account_details', { domain: d, username: u });
        if (det) openModal(d, u, det[1], true);
    }));
    tbody.querySelectorAll('button[data-delete]').forEach(b => b.addEventListener('click', async () => {
        const [d, u] = b.dataset.delete.split('|');
        const ok = await showCustomConfirm(`Xóa ${u}?`);
        if (ok) {
            await invoke('delete_account', { domain: d, username: u });
            loadPasswordTable();
        }
    }));
    tbody.querySelectorAll('button[data-copy]').forEach(b => b.addEventListener('click', async () => {
        const [d, u] = b.dataset.copy.split('|');
        const det = await invoke('get_full_account_details', { domain: d, username: u });
        if (det) navigator.clipboard.writeText(det[1]);
    }));
}

export function initPasswordManager() {
    // Nút xem quản lý mật khẩu
    document.getElementById('btn-passwords')?.addEventListener('click', () => {
        hideAllViews();
        const view = document.getElementById('view-passwords');
        if (view) { view.classList.remove('hidden'); view.classList.add('flex'); }
        document.getElementById('page-title').innerText = 'Quản lý Mật khẩu';
        loadPasswordTable();
    });

    // Listen events từ Rust
    if (listen) {
        listen('refresh-accounts', () => loadPasswordTable());

        listen('nsl-ask-save-new', async (e) => {
            const data = e.payload;
            const url = getCurrentUrl();
            let domainToSave = 'truong.hcm.edu.vn';
            if (url.includes('quanlytruonghoc')) domainToSave = 'hcm.quanlytruonghoc.edu.vn';
            else if (url.includes('taphuan')) domainToSave = 'taphuan.csdl.edu.vn';
            else if (url.includes('temis')) domainToSave = 'temis.csdl.edu.vn';
            if (invoke) await invoke('focus_main_window');
            setTimeout(async () => {
                const userAgree = await showCustomConfirm(`Phát hiện tài khoản mới: ${data.user}. Lưu không?`);
                if (userAgree) invoke('save_account', { domain: domainToSave, user: data.user, pass: data.pass }).then(() => { loadPasswordTable(); syncDataToBrowser(); });
            }, 200);
        });

        listen('nsl-ask-update', async (e) => {
            const data = e.payload;
            const url = getCurrentUrl();
            let domainToSave = 'truong.hcm.edu.vn';
            if (url.includes('quanlytruonghoc')) domainToSave = 'hcm.quanlytruonghoc.edu.vn';
            else if (url.includes('taphuan')) domainToSave = 'taphuan.csdl.edu.vn';
            else if (url.includes('temis')) domainToSave = 'temis.csdl.edu.vn';
            if (invoke) await invoke('focus_main_window');
            setTimeout(async () => {
                const userAgree = await showCustomConfirm(`Cập nhật mật khẩu cho ${data.user}?`);
                if (userAgree) invoke('save_account', { domain: domainToSave, user: data.user, pass: data.pass }).then(() => { loadPasswordTable(); syncDataToBrowser(); });
            }, 200);
        });
    }

    // Auto-load ban đầu
    setTimeout(() => loadPasswordTable(), 500);
}

export { loadPasswordTable, syncDataToBrowser };
