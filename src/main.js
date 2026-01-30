import { invoke } from '@tauri-apps/api/core';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';

// ==========================================
// 1. SIDEBAR THÔNG MINH
// ==========================================
window.toggleSidebar = async () => {
    const sb = document.getElementById('sidebar');
    const ic = document.getElementById('toggle-icon');
    const isCollapsed = sb.classList.toggle('sidebar-collapsed');
    
    ic.style.transform = isCollapsed ? 'rotate(180deg)' : 'rotate(0deg)';
    
    if (isCollapsed) {
        document.querySelectorAll('.submenu').forEach(s => s.classList.remove('open'));
    }

    try { await invoke('update_webview_layout', { sidebarWidth: isCollapsed ? 64.0 : 260.0 }); } catch (e) {}
};

window.addEventListener('resize', async () => {
    const sb = document.getElementById('sidebar');
    const w = sb.classList.contains('sidebar-collapsed') ? 64.0 : 260.0;
    try { await invoke('update_webview_layout', { sidebarWidth: w }); } catch (e) {}
});

// ==========================================
// 2. QUẢN LÝ VIEW (FIX LỖI LAYOUT)
// ==========================================
function hideAllViews() {
    // Ẩn tất cả các màn hình chức năng
    document.getElementById('view-update').classList.add('hidden');
    document.getElementById('view-update').classList.remove('flex');
    document.getElementById('view-passwords').classList.add('hidden');
    document.getElementById('view-passwords').classList.remove('flex');
    
    // Mặc định ẩn luôn vùng Browser Area để không chiếm chỗ
    document.getElementById('browser-area').classList.add('hidden');
}

// -> TAB TRÌNH DUYỆT (QLTH / CSDL)
window.loadExternalSystem = async (url, name, menuIdToUnlock) => {
    hideAllViews(); 
    
    // HIỆN LẠI VÙNG BROWSER (Để Rust biết chỗ mà vẽ, dù nó là cửa sổ con nhưng giữ logic layout)
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

// -> TAB CẬP NHẬT
window.switchToUpdate = async () => {
    await invoke('hide_embedded_view'); // Đóng trình duyệt con
    hideAllViews(); // Hàm này đã ẩn browser-area -> Nội dung sẽ tràn lên đầu trang
    
    const v = document.getElementById('view-update');
    v.classList.remove('hidden');
    v.classList.add('flex');
    document.getElementById('page-title').innerText = "Trung tâm cập nhật";
    if(document.getElementById('auto-update-btn')) runOneClickUpdate();
};

// -> TAB QUẢN LÝ MẬT KHẨU
window.switchToPasswordManager = async () => {
    await invoke('hide_embedded_view'); // Đóng trình duyệt con
    hideAllViews(); // Hàm này đã ẩn browser-area -> Nội dung sẽ tràn lên đầu trang
    
    const v = document.getElementById('view-passwords');
    v.classList.remove('hidden');
    v.classList.add('flex');
    document.getElementById('page-title').innerText = "Quản lý Mật khẩu";
    
    loadPasswordTable();
};

// ==========================================
// 3. LOGIC BẢNG MẬT KHẨU (TABLE)
// ==========================================
async function loadPasswordTable() {
    const tbody = document.getElementById('password-table-body');
    tbody.innerHTML = '<tr><td colspan="4" class="text-center text-slate-500 py-4">Đang tải...</td></tr>';
    try {
        const accounts = await invoke('get_all_accounts');
        tbody.innerHTML = '';
        if (accounts.length === 0) {
            tbody.innerHTML = '<tr><td colspan="4" class="text-center text-slate-500 py-4">Chưa có dữ liệu</td></tr>';
            return;
        }
        accounts.forEach((acc, index) => {
            const tr = document.createElement('tr');
            tr.innerHTML = `
                <td class="text-slate-400 font-mono">${index + 1}</td>
                <td class="font-medium text-white">${acc.domain}</td>
                <td class="text-cyan-300">${acc.username}</td>
                <td class="flex justify-center gap-2">
                    <button onclick="copyPass('${acc.domain}')" title="Copy" class="p-1.5 bg-slate-700 hover:bg-slate-600 rounded text-green-400">
                         <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>
                    </button>
                    <button onclick="editAccount('${acc.domain}', '${acc.username}')" title="Sửa" class="p-1.5 bg-slate-700 hover:bg-slate-600 rounded text-blue-400">
                         <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" /></svg>
                    </button>
                    <button onclick="deleteAccount('${acc.domain}')" title="Xóa" class="p-1.5 bg-slate-700 hover:bg-red-900/50 rounded text-red-400">
                         <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>
                    </button>
                </td>`;
            tbody.appendChild(tr);
        });
    } catch (e) { alert("Lỗi tải: " + e); }
}

window.copyPass = async (domain) => {
    try {
        const pass = await invoke('get_password_plaintext', { domain });
        await navigator.clipboard.writeText(pass);
        alert(`Đã copy mật khẩu của ${domain}`);
    } catch (e) { alert("Lỗi: " + e); }
};

window.deleteAccount = async (domain) => {
    if(confirm(`Xóa tài khoản ${domain}?`)) {
        try { await invoke('delete_account', { domain }); loadPasswordTable(); } catch(e) { alert("Lỗi: " + e); }
    }
};

window.openEditModal = () => openModal("", "", "");
window.editAccount = async (d, u) => {
    try { const p = await invoke('get_password_plaintext', { domain: d }); openModal(d, u, p); } 
    catch(e) { openModal(d, u, ""); }
};

function openModal(d, u, p) {
    document.getElementById('config-modal').classList.remove('hidden');
    document.getElementById('cfg-domain').value = d;
    document.getElementById('cfg-user').value = u;
    document.getElementById('cfg-pass').value = p;
    document.getElementById('cfg-domain').readOnly = (d !== ""); 
}

window.saveConfigToRust = async () => {
    const d = document.getElementById('cfg-domain').value;
    const u = document.getElementById('cfg-user').value;
    const p = document.getElementById('cfg-pass').value;
    if(!d || !u || !p) return alert("Thiếu thông tin");
    try { await invoke('save_account', { domain:d, user:u, pass:p }); document.getElementById('config-modal').classList.add('hidden'); loadPasswordTable(); } 
    catch(e) { alert("Lỗi: " + e); }
};

// --- UPDATE LOGIC ---
const logEl = document.getElementById('update-log');
const btnCheck = document.getElementById('auto-update-btn');
const btnText = document.getElementById('btn-text');
const loadingIcon = document.getElementById('loading-icon');
const progressBar = document.getElementById('progress-bar');
const statusText = document.getElementById('status-text');

function log(msg, type = 'info') {
    if (!logEl) return;
    const div = document.createElement('div');
    const time = new Date().toLocaleTimeString('vi-VN');
    div.innerHTML = `<span class="opacity-50">[${time}]</span> ${msg}`;
    if (type === 'error') div.className = "text-red-400";
    if (type === 'success') div.className = "text-green-400 font-bold";
    logEl.appendChild(div);
    logEl.scrollTop = logEl.scrollHeight;
}

async function initSystem() {
  try {
      const v = await getVersion();
      const vd = document.getElementById('current-version-display');
      if(vd) vd.innerText = `v${v}`;
      switchToUpdate();
  } catch (e) {}
}

async function runOneClickUpdate() {
    if(!btnCheck) return;
    btnCheck.disabled = true;
    loadingIcon.classList.remove('hidden');
    btnText.innerText = "Đang kiểm tra...";
    if(statusText) {
        statusText.innerText = "Đang kết nối...";
        statusText.className = "text-yellow-400 font-medium animate-pulse";
    }
    
    if(logEl) logEl.innerHTML = '';
    log(">> [AUTO] Bắt đầu quy trình cập nhật...");

    try {
        const update = await check();
        if (update) {
            log(`>> [PHÁT HIỆN] Bản mới: v${update.version}`, 'success');
            if(statusText) statusText.innerText = "Đang tải xuống...";
            btnText.innerText = "Đang tải...";
            await installUpdate(update);
        } else {
            log(">> [INFO] Bạn đang dùng phiên bản mới nhất.", 'success');
            if(statusText) {
                statusText.innerText = "Hệ thống đã cập nhật";
                statusText.className = "text-green-400 font-bold";
            }
            resetButtonState("Kiểm tra lại");
        }
    } catch (error) {
        log(`>> [LỖI] ${error}`, 'error');
        resetButtonState("Thử lại");
    }
}

async function installUpdate(update) {
    document.getElementById('progress-container').classList.remove('hidden');
    let downloaded = 0; let contentLength = 0;
    try {
        await update.downloadAndInstall((event) => {
            if (event.event === 'Started') {
                contentLength = event.data.contentLength;
            } else if (event.event === 'Progress') {
                downloaded += event.data.chunkLength;
                if (contentLength) {
                    const percent = (downloaded / contentLength) * 100;
                    progressBar.style.width = `${percent}%`;
                    btnText.innerText = `Đang tải ${Math.round(percent)}%`;
                }
            } else if (event.event === 'Finished') {
                progressBar.style.width = '100%';
            }
        });
        statusText.innerText = "Hoàn tất!";
        await new Promise(r => setTimeout(r, 1500));
        await relaunch();
    } catch (e) {
        log(`>> [LỖI] ${e}`, 'error');
        resetButtonState("Thử lại");
    }
}

function resetButtonState(text) {
    if(!btnCheck) return;
    btnCheck.disabled = false;
    loadingIcon.classList.add('hidden');
    btnText.innerText = text;
    btnCheck.onclick = async () => await runOneClickUpdate();
}

initSystem();