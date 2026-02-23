// src/menus/home.js
// Logic menu: Trang chủ + Kiểm tra & Cập nhật tự động

import { hideAllViews } from '../core/browser.js';

export function initHome() {
    document.getElementById('btn-home')?.addEventListener('click', () => {
        hideAllViews();
        document.getElementById('view-home').classList.remove('hidden');
        document.getElementById('page-title').innerText = 'Trang chủ';
    });

    // Tự động kiểm tra + cập nhật khi khởi động
    checkAndAutoUpdate();

    // Nút kiểm tra thủ công
    document.getElementById('btn-check-update')?.addEventListener('click', () => {
        checkAndAutoUpdate();
    });
}

async function checkAndAutoUpdate() {
    const btnText = document.getElementById('update-btn-text');
    const spinner = document.getElementById('update-spinner');
    const status = document.getElementById('update-status');

    if (!btnText || !status || !spinner) return;

    // Bắt đầu kiểm tra
    btnText.textContent = 'Đang kiểm tra...';
    spinner.style.display = 'inline-block';
    status.textContent = '>> Đang kết nối máy chủ...';

    try {
        const { check } = await import('@tauri-apps/plugin-updater');
        const update = await check();

        if (update) {
            // Có bản mới → tự động tải và cài đặt luôn
            btnText.textContent = 'Đang cập nhật...';
            status.innerHTML = `>> <span class="text-green-400 font-bold">Phát hiện v${update.version}</span> — Đang tải bản cập nhật...`;

            try {
                await update.downloadAndInstall((event) => {
                    if (event.event === 'Progress') {
                        status.innerHTML = `>> Đang tải bản cập nhật <span class="text-cyan-300">v${update.version}</span>...`;
                    } else if (event.event === 'Finished') {
                        status.innerHTML = '>> <span class="text-green-400">Tải xong! Đang cài đặt và khởi động lại...</span>';
                    }
                });

                spinner.style.display = 'none';
                btnText.textContent = 'Đang khởi động lại...';
                status.innerHTML = '>> <span class="text-green-400 font-bold">Cài đặt hoàn tất! Ứng dụng sẽ khởi động lại...</span>';

                const { relaunch } = await import('@tauri-apps/plugin-process');
                await relaunch();
            } catch (err) {
                spinner.style.display = 'none';
                btnText.textContent = 'Kiểm tra cập nhật';
                status.innerHTML = `>> <span class="text-red-400">Cập nhật thất bại: ${err.message || err}</span>`;
            }
        } else {
            // Đã mới nhất
            spinner.style.display = 'none';
            btnText.textContent = 'Kiểm tra cập nhật';
            status.textContent = '>> ✅ Bạn đang sử dụng phiên bản mới nhất.';
        }
    } catch (err) {
        // Lỗi mạng hoặc lỗi khác
        spinner.style.display = 'none';
        btnText.textContent = 'Kiểm tra cập nhật';
        status.innerHTML = `>> <span class="text-yellow-400">Không thể kiểm tra: ${err.message || err}</span>`;
    }
}
