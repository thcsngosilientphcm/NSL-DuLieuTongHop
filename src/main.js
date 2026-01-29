import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';

// UI Elements
const iframe = document.getElementById('main-browser');
const placeholder = document.getElementById('browser-placeholder');
const titleEl = document.getElementById('page-title');
const urlBadge = document.getElementById('url-badge');
const currentUrlEl = document.getElementById('current-url');
const viewBrowser = document.getElementById('view-browser');
const viewUpdate = document.getElementById('view-update');
const versionDisplay = document.getElementById('current-version-display');
const appVersionSide = document.getElementById('app-version');
const statusText = document.getElementById('status-text');

// Update UI Elements
const logEl = document.getElementById('update-log');
const btnCheck = document.getElementById('auto-update-btn');
const btnText = document.getElementById('btn-text');
const loadingIcon = document.getElementById('loading-icon');
const progressBar = document.getElementById('progress-bar');
const progressContainer = document.getElementById('progress-container');

// --- HÀM ĐIỀU HƯỚNG ---
window.switchToUpdate = () => {
    viewBrowser.classList.add('hidden');
    viewUpdate.classList.remove('hidden');
    viewUpdate.classList.add('flex');
    titleEl.innerText = "Trung tâm cập nhật";
    urlBadge.classList.add('hidden');
    iframe.src = ""; 
};

window.loadExternalSystem = (url, name, menuIdToUnlock) => {
    viewUpdate.classList.add('hidden');
    viewUpdate.classList.remove('flex');
    viewBrowser.classList.remove('hidden');
    placeholder.classList.add('hidden');
    
    titleEl.innerText = name;
    urlBadge.classList.remove('hidden');
    urlBadge.classList.add('flex');
    currentUrlEl.innerText = url;
    iframe.src = url;

    // Unlock menu
    document.querySelectorAll('.submenu').forEach(s => {
        if(s.id !== menuIdToUnlock) {
            s.classList.remove('open');
            s.classList.add('menu-disabled');
        }
    });
    const sub = document.getElementById(menuIdToUnlock);
    if (sub) {
        sub.classList.remove('menu-disabled');
        sub.classList.add('open');
    }
};

window.navigateIframe = (url) => {
    iframe.src = url;
    currentUrlEl.innerText = url;
};

// --- LOGIC CẬP NHẬT TỰ ĐỘNG ---

function log(msg, type = 'info') {
    const div = document.createElement('div');
    const time = new Date().toLocaleTimeString('vi-VN');
    div.innerHTML = `<span class="opacity-50">[${time}]</span> ${msg}`;
    if (type === 'error') div.className = "text-red-400";
    if (type === 'success') div.className = "text-green-400 font-bold";
    if (type === 'warn') div.className = "text-yellow-400";
    logEl.appendChild(div);
    logEl.scrollTop = logEl.scrollHeight;
}

// Hàm khởi chạy chính
async function initSystem() {
  const version = await getVersion();
  versionDisplay.innerText = `v${version}`;
  appVersionSide.innerText = `${version}`;
  
  // Mặc định vào trang Update để chạy quy trình tự động
  switchToUpdate();
  
  // Gán sự kiện cho nút (để bấm thủ công nếu muốn)
  btnCheck.onclick = async () => await runOneClickUpdate();

  // TỰ ĐỘNG CHẠY KHI MỞ APP
  setTimeout(() => {
      runOneClickUpdate();
  }, 1000); // Đợi 1s cho giao diện load xong rồi chạy
}

// Quy trình 1 nút bấm (Kiểm tra -> Tải -> Cài)
async function runOneClickUpdate() {
    // 1. Khóa nút UI
    btnCheck.disabled = true;
    loadingIcon.classList.remove('hidden');
    btnText.innerText = "Đang kiểm tra...";
    statusText.innerText = "Đang kết nối máy chủ...";
    statusText.className = "text-cyan-400 font-medium animate-pulse";
    progressContainer.classList.add('hidden');
    logEl.innerHTML = '';
    
    log(">> [AUTO] Bắt đầu quy trình cập nhật...");

    try {
        // 2. Kiểm tra
        const update = await check();
        
        if (update) {
            log(`>> [PHÁT HIỆN] Bản mới: v${update.version}`, 'success');
            log(`>> Ghi chú: ${update.body || 'Không có mô tả'}`);
            statusText.innerText = "Đang tải xuống...";
            btnText.innerText = "Đang tải cập nhật...";
            
            // 3. Tự động tải và cài đặt luôn (Không chờ user bấm nữa)
            await installUpdate(update);
            
        } else {
            log(">> [INFO] Bạn đang dùng phiên bản mới nhất.", 'success');
            statusText.innerText = "Hệ thống đã cập nhật";
            statusText.className = "text-green-400 font-bold";
            resetButtonState("Kiểm tra lại");
            
            // Nếu không có update, tự động chuyển về trang chào mừng hoặc browser (tuỳ chọn)
            // setTimeout(() => { log(">> Sẵn sàng sử dụng."); }, 1000);
        }
    } catch (error) {
        log(`>> [LỖI] Không thể kiểm tra: ${error}`, 'error');
        statusText.innerText = "Lỗi kết nối";
        statusText.className = "text-red-400 font-bold";
        resetButtonState("Thử lại");
    }
}

async function installUpdate(update) {
    progressContainer.classList.remove('hidden');
    let downloaded = 0; 
    let contentLength = 0;

    try {
        await update.downloadAndInstall((event) => {
            if (event.event === 'Started') {
                contentLength = event.data.contentLength;
                log(`>> Bắt đầu tải gói tin...`);
            } else if (event.event === 'Progress') {
                downloaded += event.data.chunkLength;
                if (contentLength) {
                    const percent = (downloaded / contentLength) * 100;
                    progressBar.style.width = `${percent}%`;
                    btnText.innerText = `Đang tải ${Math.round(percent)}%`;
                }
            } else if (event.event === 'Finished') {
                log(">> Tải xong. Đang giải nén và cài đặt...", 'success');
                progressBar.style.width = '100%';
                statusText.innerText = "Đang cài đặt...";
            }
        });

        log(">> [XONG] Cập nhật hoàn tất. Khởi động lại ngay...", 'success');
        btnText.innerText = "Khởi động lại...";
        statusText.innerText = "Hoàn tất!";
        
        await new Promise(r => setTimeout(r, 1500)); // Đợi 1.5s để user kịp đọc
        await relaunch();

    } catch (e) {
        log(`>> [LỖI CÀI ĐẶT] ${e}`, 'error');
        statusText.innerText = "Cập nhật thất bại";
        statusText.className = "text-red-400 font-bold";
        resetButtonState("Thử lại");
    }
}

function resetButtonState(text) {
    btnCheck.disabled = false;
    loadingIcon.classList.add('hidden');
    btnText.innerText = text;
    btnCheck.onclick = async () => await runOneClickUpdate();
}

// Chạy
initSystem();