import { invoke } from '@tauri-apps/api/core';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';

// --- UI Logic ---
window.toggleSidebar = async () => {
    const sb = document.getElementById('sidebar');
    const ic = document.getElementById('toggle-icon');
    const collapsed = sb.classList.toggle('sidebar-collapsed');
    ic.style.transform = collapsed ? 'rotate(180deg)' : 'rotate(0deg)';
    
    // Gọi Rust resize
    try { await invoke('update_webview_layout', { sidebarWidth: collapsed ? 64.0 : 260.0 }); } catch (e) {}
};

window.addEventListener('resize', async () => {
    const sb = document.getElementById('sidebar');
    const w = sb.classList.contains('sidebar-collapsed') ? 64.0 : 260.0;
    try { await invoke('update_webview_layout', { sidebarWidth: w }); } catch (e) {}
});

window.loadExternalSystem = async (url, name, menuIdToUnlock) => {
    document.getElementById('view-update').classList.add('hidden');
    document.getElementById('view-update').classList.remove('flex');
    document.getElementById('page-title').innerText = name;
    
    await invoke('open_secure_window', { url: url });
    
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

window.navigateRust = async (url) => {
    await invoke('navigate_webview', { url: url });
};

window.switchToUpdate = async () => {
    await invoke('hide_embedded_view');
    const v = document.getElementById('view-update');
    v.classList.remove('hidden');
    v.classList.add('flex');
    document.getElementById('page-title').innerText = "Trung tâm cập nhật";
    if(document.getElementById('auto-update-btn')) runOneClickUpdate();
};

window.openConfigModal = () => document.getElementById('config-modal').classList.remove('hidden');
window.saveConfigToRust = async () => {
    const d = document.getElementById('cfg-domain').value;
    const u = document.getElementById('cfg-user').value;
    const p = document.getElementById('cfg-pass').value;
    const res = await invoke('save_account', { domain:d, user:u, pass:p });
    alert(res);
    document.getElementById('config-modal').classList.add('hidden');
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
      if(btnCheck) btnCheck.onclick = async () => await runOneClickUpdate();
      setTimeout(runOneClickUpdate, 1000);
  } catch (e) { console.error(e); }
}

async function runOneClickUpdate() {
    if(!btnCheck) return;
    btnCheck.disabled = true;
    loadingIcon.classList.remove('hidden');
    btnText.innerText = "Đang kiểm tra...";
    if(statusText) statusText.innerText = "Đang kết nối...";
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