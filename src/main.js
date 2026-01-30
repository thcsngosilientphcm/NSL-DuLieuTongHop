import { invoke } from '@tauri-apps/api/core';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';

// 1. SIDEBAR & RESIZE
window.toggleSidebar = async () => {
    const sb = document.getElementById('sidebar');
    const ic = document.getElementById('toggle-icon');
    const collapsed = sb.classList.toggle('sidebar-collapsed');
    ic.style.transform = collapsed ? 'rotate(180deg)' : 'rotate(0deg)';
    if(collapsed) document.querySelectorAll('.submenu').forEach(s => s.classList.remove('open'));
    try { await invoke('update_webview_layout', { sidebarWidth: collapsed ? 64.0 : 260.0 }); } catch (e) {}
};
window.addEventListener('resize', async () => {
    const sb = document.getElementById('sidebar');
    const w = sb.classList.contains('sidebar-collapsed') ? 64.0 : 260.0;
    try { await invoke('update_webview_layout', { sidebarWidth: w }); } catch (e) {}
});

// 2. VIEW MANAGER
function hideAllViews() {
    document.getElementById('view-home').classList.add('hidden');
    document.getElementById('view-home').classList.remove('flex');
    document.getElementById('view-passwords').classList.add('hidden');
    document.getElementById('view-passwords').classList.remove('flex');
    document.getElementById('browser-area').classList.add('hidden');
}

window.switchToHome = async () => {
    await invoke('hide_embedded_view');
    hideAllViews();
    const v = document.getElementById('view-home');
    v.classList.remove('hidden'); v.classList.add('flex');
    document.getElementById('page-title').innerText = "Trang chủ";
};

window.loadExternalSystem = async (url, name, menuIdToUnlock) => {
    hideAllViews();
    document.getElementById('browser-area').classList.remove('hidden');
    document.getElementById('page-title').innerText = name;
    await invoke('open_secure_window', { url: url });
    const sb = document.getElementById('sidebar');
    if (!sb.classList.contains('sidebar-collapsed')) {
        document.querySelectorAll('.submenu').forEach(s => s.classList.remove('open'));
        const sub = document.getElementById(menuIdToUnlock);
        if (sub) sub.classList.add('open');
    }
};

window.navigateRust = async (url) => { await invoke('navigate_webview', { url: url }); };

window.switchToPasswordManager = async () => {
    await invoke('hide_embedded_view');
    hideAllViews();
    const v = document.getElementById('view-passwords');
    v.classList.remove('hidden'); v.classList.add('flex');
    document.getElementById('page-title').innerText = "Quản lý Mật khẩu";
    loadPasswordTable();
};

// 3. TABLE LOGIC (4 FIELDS)
async function loadPasswordTable() {
    const tbody = document.getElementById('password-table-body');
    tbody.innerHTML = '<tr><td colspan="6" class="text-center text-slate-500 py-4">Đang tải...</td></tr>';
    try {
        const accounts = await invoke('get_all_accounts');
        tbody.innerHTML = '';
        if (accounts.length === 0) {
            tbody.innerHTML = '<tr><td colspan="6" class="text-center text-slate-500 py-4">Chưa có dữ liệu</td></tr>'; return;
        }
        accounts.forEach((acc, index) => {
            const tr = document.createElement('tr');
            tr.innerHTML = `
                <td class="text-slate-400 font-mono">${index + 1}</td>
                <td class="font-medium text-white">${acc.domain}</td>
                <td class="text-cyan-300">${acc.username}</td>
                <td class="text-slate-300">${acc.cap || '-'}</td>
                <td class="text-slate-300">${acc.truong || '-'}</td>
                <td class="flex justify-center gap-2">
                    <button onclick="copyPass('${acc.domain}')" class="p-1.5 bg-slate-700 hover:bg-slate-600 rounded text-green-400">📋</button>
                    <button onclick="editAccount('${acc.domain}')" class="p-1.5 bg-slate-700 hover:bg-slate-600 rounded text-blue-400">✏️</button>
                    <button onclick="deleteAccount('${acc.domain}')" class="p-1.5 bg-slate-700 hover:bg-red-900/50 rounded text-red-400">🗑️</button>
                </td>`;
            tbody.appendChild(tr);
        });
    } catch (e) { alert("Lỗi tải: " + e); }
}

window.copyPass = async (d) => { try{await navigator.clipboard.writeText((await invoke('get_full_account_details',{domain:d}))[1]);alert("Đã copy!");}catch(e){alert(e);} };
window.deleteAccount = async (d) => { if(confirm("Xóa?")){await invoke('delete_account',{domain:d});loadPasswordTable();} };

window.editAccount = async (d) => { 
    try {
        const det = await invoke('get_full_account_details', { domain: d });
        openModal(d, det[0], det[1], det[2], det[3]);
    } catch(e) { openModal(d, "", "", "", ""); } 
};
window.openEditModal = () => openModal("", "", "", "", "");

function openModal(d, u, p, c, t) {
    document.getElementById('config-modal').classList.remove('hidden');
    document.getElementById('cfg-domain').value=d; document.getElementById('cfg-user').value=u; document.getElementById('cfg-pass').value=p;
    document.getElementById('cfg-cap').value=c||""; document.getElementById('cfg-truong').value=t||"";
    document.getElementById('cfg-domain').readOnly=(d!=="");
}

window.saveConfigToRust = async()=>{
    const d=document.getElementById('cfg-domain').value; const u=document.getElementById('cfg-user').value; const p=document.getElementById('cfg-pass').value;
    const c=document.getElementById('cfg-cap').value; const t=document.getElementById('cfg-truong').value;
    if(!d||!u||!p)return alert("Thiếu tin");
    await invoke('save_account',{domain:d,user:u,pass:p,cap:c,truong:t}); document.getElementById('config-modal').classList.add('hidden'); loadPasswordTable();
};

// 4. UPDATE LOGIC (FOOTER)
const logEl = document.getElementById('update-log');
const btnCheck = document.getElementById('auto-update-btn');
const btnText = document.getElementById('btn-text');
const loadingIcon = document.getElementById('loading-icon');
const progressBar = document.getElementById('progress-bar');
const progressContainer = document.getElementById('progress-container');

function log(msg, type = 'info') {
    if (!logEl) return;
    logEl.innerText = `>> ${msg}`;
    if (type === 'error') logEl.className = "text-[10px] font-mono text-red-400 overflow-hidden whitespace-nowrap text-ellipsis";
    else if (type === 'success') logEl.className = "text-[10px] font-mono text-green-400 font-bold overflow-hidden whitespace-nowrap text-ellipsis";
    else logEl.className = "text-[10px] font-mono text-slate-300 overflow-hidden whitespace-nowrap text-ellipsis";
}

async function initSystem() {
  try {
      const v = await getVersion();
      const vd = document.getElementById('current-version-display');
      if(vd) vd.innerText = `v${v}`;
      switchToHome();
      if(btnCheck) btnCheck.onclick = async () => await runOneClickUpdate();
      setTimeout(runOneClickUpdate, 2000);
  } catch (e) {}
}

async function runOneClickUpdate() {
    if(!btnCheck) return;
    btnCheck.disabled = true;
    loadingIcon.classList.remove('hidden');
    btnText.innerText = "Đang kiểm tra...";
    log("Đang kết nối máy chủ...");
    try {
        const update = await check();
        if (update) {
            log(`Phát hiện bản mới: v${update.version}`);
            btnText.innerText = "Đang tải...";
            await installUpdate(update);
        } else {
            log("Hệ thống đã cập nhật mới nhất.");
            resetButtonState("Kiểm tra cập nhật");
        }
    } catch (error) {
        log(`Lỗi kết nối update: ${error}`, 'error');
        resetButtonState("Thử lại");
    }
}

async function installUpdate(update) {
    if(progressContainer) progressContainer.classList.remove('hidden');
    let downloaded = 0; let contentLength = 0;
    try {
        await update.downloadAndInstall((event) => {
            if (event.event === 'Started') {
                contentLength = event.data.contentLength;
                log("Bắt đầu tải gói tin...");
            } else if (event.event === 'Progress') {
                downloaded += event.data.chunkLength;
                if (contentLength) {
                    const percent = (downloaded / contentLength) * 100;
                    if(progressBar) progressBar.style.width = `${percent}%`;
                    btnText.innerText = `Đang tải ${Math.round(percent)}%`;
                }
            } else if (event.event === 'Finished') {
                if(progressBar) progressBar.style.width = '100%';
                log("Đang cài đặt...", 'success');
            }
        });
        log("Hoàn tất! Khởi động lại...", 'success');
        await new Promise(r => setTimeout(r, 1500));
        await relaunch();
    } catch (e) {
        log(`Lỗi cài đặt: ${e}`, 'error');
        resetButtonState("Thử lại");
    }
}

function resetButtonState(text) {
    if(!btnCheck) return;
    btnCheck.disabled = false;
    loadingIcon.classList.add('hidden');
    btnText.innerText = text;
}

initSystem();