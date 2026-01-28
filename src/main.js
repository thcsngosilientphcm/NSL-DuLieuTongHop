import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { getVersion } from '@tauri-apps/api/app';

const versionEl = document.getElementById('version');
const statusEl = document.getElementById('status');
const updateBtn = document.getElementById('update-btn');

async function initSystem() {
  try {
    const version = await getVersion();
    versionEl.innerText = `${version}`;
    
    // Kiểm tra cập nhật
    const update = await check();
    
    if (update) {
      statusEl.innerHTML = `<span class="text-yellow-300 animate-pulse">Phát hiện bản mới: v${update.version}</span>`;
      updateBtn.classList.remove('hidden');
      
      updateBtn.onclick = async () => {
        statusEl.innerText = "Đang tải dữ liệu về...";
        updateBtn.disabled = true;
        updateBtn.classList.add('opacity-50', 'cursor-not-allowed');
        
        await update.downloadAndInstall((event) => {
          switch (event.event) {
            case 'Started':
              statusEl.innerText = "Bắt đầu tải xuống...";
              break;
            case 'Progress':
              // Hiển thị phần trăm nếu cần
              break;
            case 'Finished':
              statusEl.innerText = "Hoàn tất! Khởi động lại...";
              break;
          }
        });

        await relaunch();
      };
    } else {
      statusEl.innerText = "Hệ thống đã được cập nhật.";
    }
  } catch (error) {
    console.error(error);
    statusEl.innerText = "Dev Mode / Không thể kết nối GitHub.";
  }
}

initSystem();