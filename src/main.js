import { invoke } from '@tauri-apps/api/core';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';

// ==========================================
// 1. LOGIC GIAO DIỆN & SIDEBAR
// ==========================================

// Hàm thu/phóng Sidebar
window.toggleSidebar = async () => {
    const sidebar = document.getElementById('sidebar');
    const toggleIcon = document.getElementById('toggle-icon');
    
    // Toggle class CSS
    const isCollapsed = sidebar.classList.toggle('sidebar-collapsed');
    
    // Xoay icon
    toggleIcon.style.transform = isCollapsed ? 'rotate(180deg)' : 'rotate(0deg)';

    // Tính toán độ rộng mới (64px hoặc 260px)
    const newWidth = isCollapsed ? 64.0 : 260.0;
    
    // Gọi Rust để resize cái Webview bên cạnh
    try { 
        await invoke('update_webview_layout', { sidebarWidth: newWidth }); 
    } catch (e) {
        console.error("Resize error:", e);
    }
};

// Lắng nghe sự kiện resize cửa sổ chính để cập nhật lại layout
window.addEventListener('resize', async () => {
    const sidebar = document.getElementById('sidebar');
    const currentWidth = sidebar.classList.contains('sidebar-collapsed') ? 64.0 : 260.0;
    try { 
        await invoke('update_webview_layout', { sidebarWidth: currentWidth }); 
    } catch (e) {}
});

// ==========================================
// 2. LOGIC ĐIỀU HƯỚNG WEB (CHILD WEBVIEW)
// ==========================================

// Khi bấm vào menu cha (Mở trang web mới)
window.loadExternalSystem = async (url, name, menuIdToUnlock) => {
    // 1. Ẩn màn hình update đi
    document.getElementById('view-update').classList.add('hidden');
    document.getElementById('view-update').classList.remove('flex');
    
    // 2. Đổi tiêu đề header
    document.getElementById('page-title').innerText = name;

    // 3. Gọi Rust để tạo/load Webview con
    await invoke('open_secure_window', { url: url });

    // 4. Mở menu con tương ứng (Accordion)
    document.querySelectorAll('.submenu').forEach(s => {
        s.classList.remove('open');
        s.classList.add('menu-disabled');
    });
    const sub = document.getElementById(menuIdToUnlock);
    if (sub) {
        sub.classList.remove('menu-disabled');
        sub.classList.add('open');
    }
};

// Khi bấm vào menu con (Điều hướng trong webview đang mở)
window.navigateRust = async (url) => {
    await invoke('navigate_webview', { url: url });
};

// ==========================================
// 3. LOGIC CẬP NHẬT & CONFIG
// ==========================================

// Chuyển sang màn hình Update (Ẩn Webview con đi)
window.switchToUpdate = async () => {
    await invoke('hide_embedded_view');
    const v = document.getElementById('view-update');
    v.classList.remove('hidden');
    v.classList.add('flex');
    document.getElementById('page-title').innerText = "Trung tâm cập nhật";
    
    // Nếu chưa chạy update, chạy luôn
    if(document.getElementById('auto-update-btn')) runOneClickUpdate();
};

// Mở Modal Config
window.openConfigModal = () => {
    document.getElementById('config-modal').classList.remove('hidden');
    // Clear input cho an toàn
    document.getElementById('cfg-user').value = '';
    document.getElementById('cfg-pass').value = '';
};

// Lưu Config xuống Rust
window.saveConfigToRust = async () => {
    const d = document.getElementById('cfg-domain').value;
    const u = document.getElementById('cfg-user').value;
    const p = document.getElementById('cfg-pass').value;
    
    try {
        const res = await invoke('save_account', { domain:d, user:u, pass:p });
        alert(res);
        document.getElementById('config-modal').classList.add('hidden');
    } catch(e) {
        alert("Lỗi: " + e);
    }
};

// ==========================================
// 4. AUTO UPDATE SYSTEM
// ==========================================

const logEl = document.getElementById('update-log');
const btnCheck = document.getElementById('auto-update-btn');
const btnText = document.getElementById('btn-text');
const loadingIcon = document.getElementById('loading-icon');
const progressBar = document.getElementById('progress-bar');
const progressContainer = document.getElementById('progress-container');
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
      const version = await getVersion();
      const verDisplay = document.getElementById('current-version-display');
      if(verDisplay) verDisplay.innerText = `v${version}`;
      
      // Mặc định vào màn hình Update
      switchToUpdate();

      if(btnCheck) btnCheck.onclick = async () => await runOneClickUpdate();

      setTimeout(() => {
          runOneClickUpdate();
      }, 1000);
  } catch (e) {
      console.error(e);
  }
}

async function runOneClickUpdate() {
    if(!btnCheck) return;
    btnCheck.disabled = true;
    loadingIcon.classList.remove('hidden');
    btnText.innerText = "Đang kiểm tra...";
    if(statusText) statusText.innerText = "Đang kết nối...";
    progressContainer.classList.add('hidden');
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
            if(statusText) statusText.innerText = "Đã cập nhật";
            resetButtonState("Kiểm tra lại");
        }
    } catch (error) {
        log(`>> [LỖI] ${error}`, 'error');
        resetButtonState("Thử lại");
    }
}

async function installUpdate(update) {
    progressContainer.classList.remove('hidden');
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

// Khởi chạy App
initSystem();