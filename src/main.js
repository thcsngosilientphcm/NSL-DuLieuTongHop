import { invoke } from '@tauri-apps/api/core';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';

// --- PHẦN 1: QUẢN LÝ CẤU HÌNH TÀI KHOẢN ---

// Mở Modal Cài đặt (Reset form trắng để đảm bảo an toàn)
window.openConfigModal = () => {
    document.getElementById('config-modal').classList.remove('hidden');
    document.getElementById('cfg-user').value = '';
    document.getElementById('cfg-pass').value = '';
};

// Lưu mật khẩu xuống Rust (Để mã hóa và lưu vào file)
window.saveConfigToRust = async () => {
    const domain = document.getElementById('cfg-domain').value;
    const user = document.getElementById('cfg-user').value;
    const pass = document.getElementById('cfg-pass').value;

    if (!user || !pass) {
        alert("Vui lòng nhập đầy đủ Tên đăng nhập và Mật khẩu!");
        return;
    }

    try {
        // Gọi lệnh 'save_account' trong Rust
        const result = await invoke('save_account', { 
            domain: domain, 
            user: user, 
            pass: pass 
        });
        
        alert(result); // Hiện thông báo từ Rust (ví dụ: "Đã lưu thành công")
        document.getElementById('config-modal').classList.add('hidden');
    } catch (e) {
        alert("Lỗi khi lưu dữ liệu: " + e);
    }
};

// --- PHẦN 2: ĐIỀU HƯỚNG & MỞ CỬA SỔ ---

// Chuyển sang màn hình Cập nhật
window.switchToUpdate = () => {
    // Ẩn màn hình Browser
    document.getElementById('view-browser').classList.add('hidden');
    
    // Hiện màn hình Update
    const viewUpdate = document.getElementById('view-update');
    viewUpdate.classList.remove('hidden');
    viewUpdate.classList.add('flex');
    
    document.getElementById('page-title').innerText = "Trung tâm cập nhật";
    document.getElementById('url-badge').classList.add('hidden');
};

// Mở trang web (Thay thế Iframe bằng Cửa sổ Rust riêng biệt)
window.loadExternalSystem = async (url, name, menuIdToUnlock) => {
    // 1. Cập nhật giao diện chính
    document.getElementById('view-update').classList.add('hidden');
    document.getElementById('view-update').classList.remove('flex');
    document.getElementById('view-browser').classList.remove('hidden');

    // 2. Cập nhật Header
    document.getElementById('page-title').innerText = name;
    const urlBadge = document.getElementById('url-badge');
    urlBadge.classList.remove('hidden');
    urlBadge.classList.add('flex');
    document.getElementById('current-url').innerText = "Đang mở cửa sổ bảo mật...";

    // 3. GỌI RUST ĐỂ MỞ CỬA SỔ (Đây là lúc Auto-Click & Auto-Fill chạy)
    try {
        await invoke('open_secure_window', { url: url });
        
        // Cập nhật trạng thái UI sau khi mở thành công
        document.getElementById('current-url').innerText = url;
        
        // Mở khóa menu con tương ứng (để người dùng biết đang chọn cái nào)
        document.querySelectorAll('.submenu').forEach(s => {
            s.classList.remove('open');
            s.classList.add('menu-disabled');
        });
        const sub = document.getElementById(menuIdToUnlock);
        if (sub) {
            sub.classList.remove('menu-disabled');
            sub.classList.add('open');
        }

    } catch (e) {
        alert("Không thể mở cửa sổ hệ thống: " + e);
        document.getElementById('current-url').innerText = "Lỗi kết nối!";
    }
};

// Hàm hỗ trợ iframe cũ (giữ lại để tránh lỗi script trong HTML nếu còn gọi)
window.navigateIframe = (url) => {
    // Với cơ chế cửa sổ mới, hàm này có thể dùng để mở tab mới hoặc cập nhật UI
    // Hiện tại chỉ cần cập nhật text cho đẹp
    document.getElementById('current-url').innerText = url;
};


// --- PHẦN 3: HỆ THỐNG CẬP NHẬT TỰ ĐỘNG (AUTO-UPDATE) ---

// Các element UI cập nhật
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

// Hàm khởi chạy chính khi mở App
async function initSystem() {
  try {
      // Lấy phiên bản
      const version = await getVersion();
      const verDisplay = document.getElementById('current-version-display');
      const sideVer = document.getElementById('app-version');
      
      if(verDisplay) verDisplay.innerText = `v${version}`;
      if(sideVer) sideVer.innerText = `${version}`;
      
      // Mặc định vào trang Update để chạy quy trình kiểm tra
      switchToUpdate();
      
      // Gán sự kiện cho nút (để bấm thủ công)
      if(btnCheck) {
          btnCheck.onclick = async () => await runOneClickUpdate();
      }

      // TỰ ĐỘNG CHẠY SAU 1 GIÂY
      setTimeout(() => {
          runOneClickUpdate();
      }, 1000);

  } catch (e) {
      console.error("Init Error:", e);
  }
}

// Quy trình 1 nút bấm (Kiểm tra -> Tải -> Cài)
async function runOneClickUpdate() {
    if(!btnCheck) return;

    // Khóa nút
    btnCheck.disabled = true;
    loadingIcon.classList.remove('hidden');
    btnText.innerText = "Đang kiểm tra...";
    if(statusText) {
        statusText.innerText = "Đang kết nối...";
        statusText.className = "text-cyan-400 font-medium animate-pulse";
    }
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
            if(statusText) {
                statusText.innerText = "Đã cập nhật";
                statusText.className = "text-green-400 font-bold";
            }
            resetButtonState("Kiểm tra lại");
        }
    } catch (error) {
        log(`>> [LỖI] Không thể kiểm tra: ${error}`, 'error');
        if(statusText) {
            statusText.innerText = "Lỗi kết nối";
            statusText.className = "text-red-400 font-bold";
        }
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
                log(`>> Bắt đầu tải...`);
            } else if (event.event === 'Progress') {
                downloaded += event.data.chunkLength;
                if (contentLength) {
                    const percent = (downloaded / contentLength) * 100;
                    progressBar.style.width = `${percent}%`;
                    btnText.innerText = `Đang tải ${Math.round(percent)}%`;
                }
            } else if (event.event === 'Finished') {
                log(">> Tải xong. Đang cài đặt...", 'success');
                progressBar.style.width = '100%';
            }
        });

        log(">> [XONG] Đang khởi động lại...", 'success');
        if(statusText) statusText.innerText = "Hoàn tất!";
        
        await new Promise(r => setTimeout(r, 1500));
        await relaunch();

    } catch (e) {
        log(`>> [LỖI] ${e}`, 'error');
        if(statusText) statusText.innerText = "Thất bại";
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