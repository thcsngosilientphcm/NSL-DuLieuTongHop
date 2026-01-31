import { invoke } from '@tauri-apps/api/core';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';

// ==========================================
// 1. SIDEBAR & UI LOGIC
// ==========================================
window.toggleSidebar = async () => {
    const sb = document.getElementById('sidebar');
    const ic = document.getElementById('toggle-icon');
    const collapsed = sb.classList.toggle('sidebar-collapsed');
    
    ic.style.transform = collapsed ? 'rotate(180deg)' : 'rotate(0deg)';
    
    // Đóng hết submenu khi thu gọn
    if(collapsed) {
        document.querySelectorAll('.submenu').forEach(s => s.classList.remove('open'));
    }

    try { 
        await invoke('update_webview_layout', { sidebarWidth: collapsed ? 64.0 : 260.0 }); 
    } catch (e) {}
};

window.addEventListener('resize', async () => {
    const sb = document.getElementById('sidebar');
    const w = sb.classList.contains('sidebar-collapsed') ? 64.0 : 260.0;
    try { await invoke('update_webview_layout', { sidebarWidth: w }); } catch (e) {}
});

// ==========================================
// 2. VIEW MANAGER (CHUYỂN TAB)
// ==========================================
function hideAllViews() {
    // Ẩn view Home
    document.getElementById('view-home').classList.add('hidden');
    document.getElementById('view-home').classList.remove('flex');
    
    // Ẩn view Passwords
    document.getElementById('view-passwords').classList.add('hidden');
    document.getElementById('view-passwords').classList.remove('flex');

    // Ẩn vùng Browser
    document.getElementById('browser-area').classList.add('hidden');
}

// -> TAB: TRANG CHỦ
window.switchToHome = async () => {
    await invoke('hide_embedded_view'); // Đóng browser con
    hideAllViews();
    
    const v = document.getElementById('view-home');
    v.classList.remove('hidden'); 
    v.classList.add('flex');
    document.getElementById('page-title').innerText = "Trang chủ";
};

// -> TAB: TRÌNH DUYỆT (QLTH / CSDL)
window.loadExternalSystem = async (url, name, menuIdToUnlock) => {
    hideAllViews();
    
    // Hiện vùng browser để Rust vẽ lên
    document.getElementById('browser-area').classList.remove('hidden');
    document.getElementById('page-title').innerText = name;
    
    await invoke('open_secure_window', { url: url });
    
    // Mở submenu tương ứng
    const sb = document.getElementById('sidebar');
    if (!sb.classList.contains('sidebar-collapsed')) {
        document.querySelectorAll('.submenu').forEach(s => s.classList.remove('open'));
        const sub = document.getElementById(menuIdToUnlock);
        if (sub) sub.classList.add('open');
    }
};

window.navigateRust = async (url) => { await invoke('navigate_webview', { url: url }); };

// -> TAB: QUẢN LÝ MẬT KHẨU
window.switchToPasswordManager = async () => {
    await invoke('hide_embedded_view');
    hideAllViews();
    
    const v = document.getElementById('view-passwords');
    v.classList.remove('hidden'); 
    v.classList.add('flex');
    document.getElementById('page-title').innerText = "Quản lý Mật khẩu";
    
    loadPasswordTable();
};

// ==========================================
// 3. LOGIC BẢNG MẬT KHẨU (HỖ TRỢ ĐA TÀI KHOẢN)
// ==========================================
async function loadPasswordTable() {
    const tbody = document.getElementById('password-table-body');
    tbody.innerHTML = '<tr><td colspan="6" class="text-center text-slate-500 py-4">Đang tải...</td></tr>';
    
    try {
        const accounts = await invoke('get_all_accounts');
        tbody.innerHTML = '';
        
        if (accounts.length === 0) {
            tbody.innerHTML = '<tr><td colspan="6" class="text-center text-slate-500 py-4">Chưa có dữ liệu</td></tr>';
            return;
        }

        accounts.forEach((acc, index) => {
            const tr = document.createElement('tr');
            // Lưu ý: acc.username là khóa định danh cùng với domain
            tr.innerHTML = `
                <td class="text-slate-400 font-mono">${index + 1}</td>
                <td class="font-medium text-white">${acc.domain}</td>
                <td class="text-cyan-300 font-bold">${acc.username}</td>
                <td class="text-slate-300">${acc.cap || '-'}</td>
                <td class="text-slate-300">${acc.truong || '-'}</td>
                <td class="flex justify-center gap-2">
                    <button onclick="copyPass('${acc.domain}', '${acc.username}')" title="Copy Mật khẩu" class="p-1.5 bg-slate-700 hover:bg-slate-600 rounded text-green-400">
                        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>
                    </button>
                    <button onclick="editAccount('${acc.domain}', '${acc.username}')" title="Sửa" class="p-1.5 bg-slate-700 hover:bg-slate-600 rounded text-blue-400">
                        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" /></svg>
                    </button>
                    <button onclick="deleteAccount('${acc.domain}', '${acc.username}')" title="Xóa" class="p-1.5 bg-slate-700 hover:bg-red-900/50 rounded text-red-400">
                        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>
                    </button>
                </td>
            `;
            tbody.appendChild(tr);
        });
    } catch (e) {
        alert("Lỗi tải dữ liệu: " + e);
    }
}

// COPY MẬT KHẨU
window.copyPass = async (domain, username) => {
    try {
        // Gọi Rust lấy full info (trả về mảng [user, pass, cap, truong])
        const details = await invoke('get_full_account_details', { domain: domain, username: username });
        await navigator.clipboard.writeText(details[1]); // Pass là phần tử thứ 2
        alert(`Đã copy mật khẩu của ${username}`);
    } catch (e) { alert("Lỗi: " + e); }
};

// XÓA TÀI KHOẢN
window.deleteAccount = async (domain, username) => {
    if(confirm(`Bạn có chắc muốn xóa tài khoản ${username} của trang ${domain}?`)) {
        try {
            await invoke('delete_account', { domain: domain, username: username });
            loadPasswordTable();
        } catch(e) { alert("Lỗi: " + e); }
    }
};

// SỬA TÀI KHOẢN (Hiển thị Modal)
window.editAccount = async (domain, username) => {
    try {
        const det = await invoke('get_full_account_details', { domain: domain, username: username });
        // det = [user, pass, cap, truong]
        openModal(domain, det[0], det[1], det[2], det[3]);
        
        // Khi sửa, KHÔNG cho đổi tên đăng nhập (vì nó là khóa chính)
        const userInput = document.getElementById('cfg-user');
        userInput.readOnly = true;
        userInput.classList.add('opacity-50', 'cursor-not-allowed');
    } catch(e) { 
        openModal(domain, "", "", "", ""); 
    }
};

// THÊM MỚI (Hiển thị Modal)
window.openEditModal = () => {
    openModal("", "", "", "", "");
    // Khi thêm mới, cho phép nhập User
    const userInput = document.getElementById('cfg-user');
    userInput.readOnly = false;
    userInput.classList.remove('opacity-50', 'cursor-not-allowed');
};

// Helper mở Modal
function openModal(d, u, p, c, t) {
    document.getElementById('config-modal').classList.remove('hidden');
    
    document.getElementById('cfg-domain').value = d;
    document.getElementById('cfg-user').value = u;
    document.getElementById('cfg-pass').value = p;
    document.getElementById('cfg-cap').value = c || "";
    document.getElementById('cfg-truong').value = t || "";
    
    // Nếu đang sửa (domain có giá trị), khóa ô Domain
    document.getElementById('cfg-domain').readOnly = (d !== "");
}

// LƯU LẠI (Gửi về Rust)
window.saveConfigToRust = async () => {
    const d = document.getElementById('cfg-domain').value;
    const u = document.getElementById('cfg-user').value;
    const p = document.getElementById('cfg-pass').value;
    const c = document.getElementById('cfg-cap').value;
    const t = document.getElementById('cfg-truong').value;

    if(!d || !u || !p) { 
        alert("Vui lòng nhập đủ: Website, Tài khoản và Mật khẩu"); 
        return; 
    }
    
    try {
        await invoke('save_account', { domain:d, user:u, pass:p, cap:c, truong:t });
        document.getElementById('config-modal').classList.add('hidden');
        loadPasswordTable();
    } catch(e) { alert("Lỗi: " + e); }
};

// ==========================================
// 4. HỆ THỐNG CẬP NHẬT TỰ ĐỘNG
// ==========================================
const logEl = document.getElementById('update-log');
const btnCheck = document.getElementById('auto-update-btn');
const btnText = document.getElementById('btn-text');
const loadingIcon = document.getElementById('loading-icon');
const progressBar = document.getElementById('progress-bar');
const progressContainer = document.getElementById('progress-container');

function log(msg, type = 'info') {
    if (!logEl) return;
    logEl.innerText = `>> ${msg}`;
    
    if (type === 'error') {
        logEl.className = "text-[10px] font-mono text-red-400 overflow-hidden whitespace-nowrap text-ellipsis";
    } else if (type === 'success') {
        logEl.className = "text-[10px] font-mono text-green-400 font-bold overflow-hidden whitespace-nowrap text-ellipsis";
    } else {
        logEl.className = "text-[10px] font-mono text-slate-300 overflow-hidden whitespace-nowrap text-ellipsis";
    }
}

async function initSystem() {
  try {
      const v = await getVersion();
      const vd = document.getElementById('current-version-display');
      if(vd) vd.innerText = `v${v}`;
      
      // Mặc định vào Trang chủ
      switchToHome();

      // Gắn sự kiện nút cập nhật
      if(btnCheck) btnCheck.onclick = async () => await runOneClickUpdate();

      // Tự động kiểm tra sau 2 giây
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
            log(`Phát hiện bản mới: v${update.version}`, 'success');
            btnText.innerText = "Đang tải...";
            await installUpdate(update);
        } else {
            log("Hệ thống đã cập nhật mới nhất.", 'success');
            resetButtonState("Kiểm tra cập nhật");
        }
    } catch (error) {
        log(`Lỗi kết nối update: ${error}`, 'error');
        resetButtonState("Thử lại");
    }
}

async function installUpdate(update) {
    if(progressContainer) progressContainer.classList.remove('hidden');
    
    let downloaded = 0; 
    let contentLength = 0;
    
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

// Khởi chạy
initSystem();