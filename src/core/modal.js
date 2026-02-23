// src/core/modal.js
// Logic modal: config modal, confirm dialog

const invoke = window.__TAURI__?.core?.invoke;

let isEditingMode = false;

function setValue(id, val) {
    const el = document.getElementById(id);
    if (el) el.value = val || "";
}

export function openModal(d, u, p, isEdit = false) {
    isEditingMode = isEdit;
    document.getElementById('config-modal').classList.remove('hidden');
    const title = document.querySelector('#config-modal h3');
    if (title) title.innerText = isEdit ? "Cập nhật Tài khoản" : "Thêm Tài khoản Mới";
    setValue('cfg-domain', d); setValue('cfg-user', u); setValue('cfg-pass', p);
    const dIn = document.getElementById('cfg-domain');
    if (dIn) dIn.readOnly = d !== '';
}

export function showCustomConfirm(message) {
    return new Promise((resolve) => {
        const modal = document.getElementById('confirm-modal');
        const msgEl = document.getElementById('confirm-msg');
        const btnYes = document.getElementById('confirm-yes');
        const btnNo = document.getElementById('confirm-no');
        if (!modal || !msgEl) { resolve(confirm(message)); return; }
        msgEl.innerText = message;
        modal.classList.remove('hidden');
        setTimeout(() => { modal.classList.remove('opacity-0'); modal.classList.add('translate-y-4'); }, 10);
        const close = (result) => {
            modal.classList.add('opacity-0'); modal.classList.remove('translate-y-4');
            setTimeout(() => { modal.classList.add('hidden'); resolve(result); }, 300);
        };
        btnYes.onclick = () => close(true);
        btnNo.onclick = () => close(false);
    });
}

export function initModals(onSaved) {
    document.getElementById('add-account-btn')?.addEventListener('click', () => {
        openModal('', '', '', false);
    });

    document.getElementById('cfg-cancel')?.addEventListener('click', () => {
        document.getElementById('config-modal').classList.add('hidden');
    });

    document.getElementById('cfg-save')?.addEventListener('click', () => {
        saveConfigToRust(onSaved);
    });
}

async function saveConfigToRust(onSaved) {
    const d = document.getElementById('cfg-domain').value;
    const u = document.getElementById('cfg-user').value;
    const p = document.getElementById('cfg-pass').value;
    if (d && u && p) {
        await invoke('save_account', { domain: d, user: u, pass: p });
        document.getElementById('config-modal').classList.add('hidden');
        if (onSaved) onSaved();
    }
}

// Export cho window scope (dùng từ bên ngoài nếu cần)
window.saveConfigToRust = saveConfigToRust;
