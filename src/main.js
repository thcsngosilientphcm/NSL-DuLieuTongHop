import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';

// --- ELEMENTS ---
const iframe = document.getElementById('main-browser');
const placeholder = document.getElementById('browser-placeholder');
const titleEl = document.getElementById('page-title');
const urlBadge = document.getElementById('url-badge');
const currentUrlEl = document.getElementById('current-url');

const viewBrowser = document.getElementById('view-browser');
const viewUpdate = document.getElementById('view-update');
const versionDisplay = document.getElementById('current-version-display');
const appVersionSide = document.getElementById('app-version');

// --- HÀM ĐIỀU HƯỚNG ---

// 1. Chuyển sang View Cập nhật
window.switchToUpdate = () => {
    viewBrowser.classList.add('hidden');
    viewUpdate.classList.remove('hidden');
    viewUpdate.classList.add('flex');
    
    titleEl.innerText = "Trung tâm cập nhật";
    urlBadge.classList.add('hidden');
    iframe.src = ""; // Tắt iframe để nhẹ máy
};

// 2. Chuyển sang View Trình duyệt (Load Web)
window.loadExternalSystem = (url, name, menuIdToUnlock) => {
    // Hiện View Browser
    viewUpdate.classList.add('hidden');
    viewUpdate.classList.remove('flex');
    viewBrowser.classList.remove('hidden');
    placeholder.classList.add('hidden'); // Ẩn màn hình chờ
    
    // Cập nhật thông tin Header
    titleEl.innerText = name;
    urlBadge.classList.remove('hidden');
    urlBadge.classList.add('flex');
    currentUrlEl.innerText = url;
    
    // Load trang web
    iframe.src = url;

    // --- LOGIC MỞ KHÓA MENU CON ---
    // Đóng tất cả các menu con khác trước
    document.querySelectorAll('.submenu').forEach(s => {
        if(s.id !== menuIdToUnlock) {
            s.classList.remove('open');
            s.classList.add('menu-disabled'); // Khóa lại
        }
    });

    // Mở khóa menu con hiện tại
    const sub = document.getElementById(menuIdToUnlock);
    if (sub) {
        sub.classList.remove('menu-disabled'); // Sáng lên
        sub.classList.add('open');             // Sổ xuống
    }
};

// 3. Điều hướng Iframe khi bấm menu con
window.navigateIframe = (url) => {
    iframe.src = url;
    currentUrlEl.innerText = url;
};


// --- LOGIC CẬP NHẬT (GIỮ NGUYÊN) ---
const logEl = document.getElementById('update-log');
const btnCheck = document.getElementById('manual-check-btn');
const btnText = document.getElementById('btn-text');
const loadingIcon = document.getElementById('loading-icon');
const progressBar = document.getElementById('progress-bar');
const progressContainer = document.getElementById('progress-container');

function log(msg, type = 'info') {
    const div = document.createElement('div');
    const time = new Date().toLocaleTimeString('vi-VN');
    div.innerHTML = `<span class="opacity-50">[${time}]</span> ${msg}`;
    if (type === 'error') div.className = "text-red-400";
    if (type === 'success') div.className = "text-green-400 font-bold";
    logEl.appendChild(div);
    logEl.scrollTop = logEl.scrollHeight;
}

async function initSystem() {
  const version = await getVersion();
  versionDisplay.innerText = `v${version}`;
  appVersionSide.innerText = `v${version}`;
  
  // Mặc định vào trang cập nhật trước
  switchToUpdate();

  btnCheck.onclick = async () => await runUpdateCheck();
}

async function runUpdateCheck() {
    btnCheck.disabled = true;
    loadingIcon.classList.remove('hidden');
    btnText.innerText = "Đang kết nối...";
    progressContainer.classList.add('hidden');
    logEl.innerHTML = '';
    log(">> Bắt đầu kiểm tra...");

    try {
        const update = await check();
        if (update) {
            log(`>> Có bản mới: v${update.version}`, 'success');
            btnText.innerText = `Cài đặt v${update.version}`;
            loadingIcon.classList.add('hidden');
            btnCheck.disabled = false;
            btnCheck.classList.add('from-green-600', 'to-emerald-600', 'animate-pulse');
            btnCheck.onclick = async () => await installUpdate(update);
        } else {
            log(">> Đang là phiên bản mới nhất.", 'success');
            resetButtonState();
        }
    } catch (error) {
        log(`>> LỖI: ${error}`, 'error');
        resetButtonState();
    }
}

async function installUpdate(update) {
    btnCheck.disabled = true;
    btnText.innerText = "Đang tải...";
    loadingIcon.classList.remove('hidden');
    progressContainer.classList.remove('hidden');
    let downloaded = 0; let contentLength = 0;

    try {
        await update.downloadAndInstall((event) => {
            if (event.event === 'Started') {
                contentLength = event.data.contentLength;
                log(`>> Bắt đầu tải...`);
            } else if (event.event === 'Progress') {
                downloaded += event.data.chunkLength;
                if (contentLength) progressBar.style.width = `${(downloaded / contentLength) * 100}%`;
            } else if (event.event === 'Finished') {
                log(">> Tải xong. Giải nén...", 'success');
                progressBar.style.width = '100%';
            }
        });
        log(">> Thành công! Khởi động lại...", 'success');
        await new Promise(r => setTimeout(r, 1000));
        await relaunch();
    } catch (e) {
        log(`>> LỖI CÀI ĐẶT: ${e}`, 'error');
        resetButtonState();
    }
}

function resetButtonState() {
    btnCheck.disabled = false;
    btnCheck.classList.remove('from-green-600', 'to-emerald-600', 'animate-pulse');
    loadingIcon.classList.add('hidden');
    btnText.innerText = "Kiểm tra lại";
    btnCheck.onclick = async () => await runUpdateCheck();
}

initSystem();