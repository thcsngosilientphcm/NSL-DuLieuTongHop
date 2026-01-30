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
    
    // KHI THU NHỎ -> ĐÓNG HẾT SUBMENU
    if (isCollapsed) {
        document.querySelectorAll('.submenu').forEach(s => {
            s.classList.remove('open');
            // Logic CSS đã handle việc display:none
        });
    }

    // Resize Webview
    try { await invoke('update_webview_layout', { sidebarWidth: isCollapsed ? 64.0 : 260.0 }); } catch (e) {}
};

window.addEventListener('resize', async () => {
    const sb = document.getElementById('sidebar');
    const w = sb.classList.contains('sidebar-collapsed') ? 64.0 : 260.0;
    try { await invoke('update_webview_layout', { sidebarWidth: w }); } catch (e) {}
});

// ==========================================
// 2. QUẢN LÝ VIEW (CHUYỂN TAB)
// ==========================================
function hideAllViews() {
    document.getElementById('view-update').classList.remove('flex');
    document.getElementById('view-update').classList.add('hidden');
    
    document.getElementById('view-passwords').classList.remove('flex');
    document.getElementById('view-passwords').classList.add('hidden');
}

// -> TAB TRÌNH DUYỆT (QLTH / CSDL)
window.loadExternalSystem = async (url, name, menuIdToUnlock) => {
    hideAllViews(); // Ẩn các view HTML
    document.getElementById('page-title').innerText = name;
    
    // Mở Webview Rust
    await invoke('open_secure_window', { url: url });
    
    // Mở Submenu tương ứng
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
    await invoke('hide_embedded_view'); // ĐÓNG WEBVIEW RUST
    hideAllViews();
    
    const v = document.getElementById('view-update');
    v.classList.remove('hidden');
    v.classList.add('flex');
    document.getElementById('page-title').innerText = "Trung tâm cập nhật";
    if(document.getElementById('auto-update-btn')) runOneClickUpdate();
};

// -> TAB QUẢN LÝ MẬT KHẨU (MỚI)
window.switchToPasswordManager = async () => {
    await invoke('hide_embedded_view'); // ĐÓNG WEBVIEW RUST
    hideAllViews();
    
    const v = document.getElementById('view-passwords');
    v.classList.remove('hidden');
    v.classList.add('flex'); // Flex để dùng layout dọc
    document.getElementById('page-title').innerText = "Quản lý Mật khẩu";
    
    loadPasswordTable(); // Tải dữ liệu
};

// ==========================================
// 3. LOGIC BẢNG MẬT KHẨU (TABLE)
// ==========================================
async function loadPasswordTable() {
    const tbody = document.getElementById('password-table-body');
    tbody.innerHTML = '<tr><td colspan="4" class="text-center text-slate-500 py-4">Đang tải...</td></tr>';
    
    try {
        const accounts = await invoke('get_all_accounts'); // Gọi Rust lấy list
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
                    <button onclick="copyPass('${acc.domain}')" title="Copy Mật khẩu" class="p-1.5 bg-slate-700 hover:bg-slate-600 rounded text-green-400">
                        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" /></svg>
                    </button>
                    <button onclick="editAccount('${acc.domain}', '${acc.username}')" title="Chỉnh sửa" class="p-1.5 bg-slate-700 hover:bg-slate-600 rounded text-blue-400">
                        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" /></svg>
                    </button>
                    <button onclick="deleteAccount('${acc.domain}')" title="Xóa" class="p-1.5 bg-slate-700 hover:bg-red-900/50 rounded text-red-400">
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

// Hành động: Copy Pass
window.copyPass = async (domain) => {
    try {
        const pass = await invoke('get_password_plaintext', { domain });
        await navigator.clipboard.writeText(pass);
        alert(`Đã copy mật khẩu của ${domain}`);
    } catch (e) { alert("Lỗi: " + e); }
};

// Hành động: Xóa
window.deleteAccount = async (domain) => {
    if(confirm(`Bạn chắc chắn muốn xóa tài khoản của ${domain}?`)) {
        try {
            await invoke('delete_account', { domain });
            loadPasswordTable(); // Reload bảng
        } catch(e) { alert("Lỗi: " + e); }
    }
};

// Hành động: Sửa / Thêm
window.openEditModal = () => openModal("", "", "");
window.editAccount = async (domain, user) => {
    // Để edit, ta cần lấy pass cũ giải mã ra để điền vào ô input (hoặc để trống nếu ko muốn đổi)
    // Ở đây ta cứ lấy pass ra hiển thị cho tiện
    try {
        const pass = await invoke('get_password_plaintext', { domain });
        openModal(domain, user, pass);
    } catch(e) { openModal(domain, user, ""); }
};

function openModal(d, u, p) {
    document.getElementById('config-modal').classList.remove('hidden');
    document.getElementById('cfg-domain').value = d;
    document.getElementById('cfg-user').value = u;
    document.getElementById('cfg-pass').value = p;
    // Nếu đang sửa thì disable domain (vì nó là Key) - Hoặc cho sửa thoải mái cũng đc
    document.getElementById('cfg-domain').readOnly = (d !== ""); 
}

window.saveConfigToRust = async () => {
    const d = document.getElementById('cfg-domain').value;
    const u = document.getElementById('cfg-user').value;
    const p = document.getElementById('cfg-pass').value;
    if(!d || !u || !p) { alert("Vui lòng nhập đủ thông tin"); return; }
    
    try {
        await invoke('save_account', { domain:d, user:u, pass:p });
        document.getElementById('config-modal').classList.add('hidden');
        loadPasswordTable(); // Refresh bảng
    } catch(e) { alert("Lỗi: " + e); }
};

// --- AUTO UPDATE & INIT (Giữ nguyên) ---
async function initSystem() {
  try {
      const v = await getVersion();
      const vd = document.getElementById('current-version-display');
      if(vd) vd.innerText = `v${v}`;
      switchToUpdate(); // Mặc định vào trang update
  } catch (e) {}
}
// (Các hàm update cũ giữ nguyên, copy lại từ bản trước hoặc để tôi viết lại ngắn gọn)
const btnCheck = document.getElementById('auto-update-btn');
const statusText = document.getElementById('btn-text'); // Sửa lại trỏ đúng ID
async function runOneClickUpdate() {
    if(!btnCheck) return;
    btnCheck.disabled = true;
    if(statusText) statusText.innerText = "Đang kiểm tra...";
    try {
        const update = await check();
        if (update) {
            statusText.innerText = "Đang tải...";
            await update.downloadAndInstall();
            await relaunch();
        } else {
            statusText.innerText = "Đã cập nhật mới nhất";
            setTimeout(() => { btnCheck.disabled = false; statusText.innerText = "Kiểm tra lại"; }, 2000);
        }
    } catch (e) {
        statusText.innerText = "Lỗi kết nối";
        btnCheck.disabled = false;
    }
}

initSystem();