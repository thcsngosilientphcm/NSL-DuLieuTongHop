import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';

// Các phần tử UI mới
const logEl = document.getElementById('update-log');
const btnCheck = document.getElementById('manual-check-btn');
const btnText = document.getElementById('btn-text');
const loadingIcon = document.getElementById('loading-icon');
const progressBar = document.getElementById('progress-bar');
const progressContainer = document.getElementById('progress-container');
const versionDisplay = document.getElementById('current-version-display');
const appVersionSide = document.getElementById('app-version');

// Hàm ghi log ra màn hình console giả lập
function log(msg, type = 'info') {
    const div = document.createElement('div');
    const time = new Date().toLocaleTimeString('vi-VN');
    div.innerHTML = `<span class="opacity-50">[${time}]</span> ${msg}`;
    
    if (type === 'error') div.className = "text-red-400";
    if (type === 'success') div.className = "text-green-400 font-bold";
    if (type === 'warn') div.className = "text-yellow-400";
    
    logEl.appendChild(div);
    logEl.scrollTop = logEl.scrollHeight; // Tự cuộn xuống dưới cùng
}

async function initSystem() {
  // 1. Lấy version hiện tại
  const version = await getVersion();
  versionDisplay.innerText = `v${version}`;
  appVersionSide.innerText = `v${version}`;
  
  // 2. Gán sự kiện cho nút Cập Nhật
  btnCheck.onclick = async () => {
    await runUpdateCheck();
  };
}

async function runUpdateCheck() {
    // UI: Đang tải
    btnCheck.disabled = true;
    btnCheck.classList.add('opacity-70', 'cursor-not-allowed');
    loadingIcon.classList.remove('hidden');
    btnText.innerText = "Đang kết nối GitHub...";
    progressContainer.classList.add('hidden');
    logEl.innerHTML = ''; // Xóa log cũ
    
    log(">> Bắt đầu kiểm tra máy chủ cập nhật...");

    try {
        const update = await check();
        
        if (update) {
            log(`>> Phát hiện phiên bản mới: v${update.version}`, 'success');
            log(`>> Ghi chú: ${update.body || 'Không có mô tả.'}`);
            
            // Đổi nút thành "Cài đặt ngay"
            btnText.innerText = `Cài đặt v${update.version}`;
            loadingIcon.classList.add('hidden');
            btnCheck.disabled = false;
            btnCheck.classList.remove('opacity-70', 'cursor-not-allowed');
            btnCheck.classList.remove('from-blue-600', 'to-cyan-600');
            btnCheck.classList.add('from-green-600', 'to-emerald-600', 'animate-pulse'); // Đổi màu nút

            // Gán hành động tiếp theo cho nút là Tải về
            btnCheck.onclick = async () => {
                await installUpdate(update);
            };

        } else {
            log(">> Hệ thống của bạn đang ở phiên bản mới nhất.", 'success');
            resetButtonState();
        }
    } catch (error) {
        log(`>> LỖI: Không thể kiểm tra cập nhật.\n${error}`, 'error');
        resetButtonState();
    }
}

async function installUpdate(update) {
    // UI: Chuyển sang chế độ tải
    btnCheck.disabled = true;
    btnText.innerText = "Đang tải xuống...";
    loadingIcon.classList.remove('hidden');
    progressContainer.classList.remove('hidden');
    
    let downloaded = 0;
    let contentLength = 0;

    try {
        await update.downloadAndInstall((event) => {
            switch (event.event) {
                case 'Started':
                    contentLength = event.data.contentLength;
                    log(`>> Bắt đầu tải gói tin (${(contentLength/1024/1024).toFixed(2)} MB)...`);
                    break;
                case 'Progress':
                    downloaded += event.data.chunkLength;
                    if (contentLength) {
                        const percent = (downloaded / contentLength) * 100;
                        progressBar.style.width = `${percent}%`;
                        // Cập nhật text trên nút để người dùng đỡ sốt ruột
                        btnText.innerText = `Đang tải ${Math.round(percent)}%`;
                    }
                    break;
                case 'Finished':
                    log(">> Tải xuống hoàn tất. Đang giải nén...", 'success');
                    progressBar.style.width = '100%';
                    break;
            }
        });

        log(">> Cập nhật thành công! Đang khởi động lại...", 'success');
        btnText.innerText = "Khởi động lại...";
        await new Promise(r => setTimeout(r, 1000)); // Đợi 1s cho đẹp
        await relaunch();

    } catch (e) {
        log(`>> LỖI CÀI ĐẶT: ${e}`, 'error');
        resetButtonState();
        progressContainer.classList.add('hidden');
    }
}

function resetButtonState() {
    btnCheck.disabled = false;
    btnCheck.classList.remove('opacity-70', 'cursor-not-allowed', 'from-green-600', 'to-emerald-600', 'animate-pulse');
    btnCheck.classList.add('from-blue-600', 'to-cyan-600');
    loadingIcon.classList.add('hidden');
    btnText.innerText = "Kiểm tra lại";
    
    // Reset click event về kiểm tra
    btnCheck.onclick = async () => {
        await runUpdateCheck();
    };
}

// Khởi chạy
initSystem();