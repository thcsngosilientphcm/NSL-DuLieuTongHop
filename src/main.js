// src/main.js
const invoke = window.__TAURI__?.core?.invoke;
const listen = window.__TAURI__?.event?.listen;

let currentMainUrl = "";
let isEditingMode = false;
let resizeObserver = null;

const SIDEBAR_WIDTH_OPEN = 310.0;
const SIDEBAR_WIDTH_COLLAPSED = 64.0;

function log(...args) { if (console && console.log) console.log(...args); }

// --- HÀM GỬI TOẠ ĐỘ CHO RUST (ĐỂ VẼ WEBVIEW ĐÈ LÊN) ---
function updateBrowserBounds() {
    const mount = document.getElementById('browser-mount-point');
    if (!mount || mount.classList.contains('hidden')) return;

    // Lấy toạ độ và kích thước thực tế của thẻ div trong cửa sổ
    const rect = mount.getBoundingClientRect();
    
    if (invoke) {
        invoke('update_embedded_browser_bounds', {
            x: Math.round(rect.x),
            y: Math.round(rect.y),
            width: Math.round(rect.width),
            height: Math.round(rect.height)
        }).catch(console.error);
    }
}

// ... Helper functions ...
function hideAllViews() {
  ['view-home','view-passwords'].forEach(id=>{
    document.getElementById(id).classList.add('hidden');
  });
  
  // Ẩn browser mount point và báo Rust ẩn Webview đi
  const mount = document.getElementById('browser-mount-point');
  if(mount) mount.classList.add('hidden');
  
  if(invoke) invoke('hide_embedded_browser').catch(()=>{});
}

function setValue(id, val) { const el = document.getElementById(id); if (el) el.value = val || ""; }
function openModal(d, u, p, isEdit = false) {
  isEditingMode = isEdit;
  document.getElementById('config-modal').classList.remove('hidden');
  const title = document.querySelector('#config-modal h3');
  if(title) title.innerText = isEdit ? "Cập nhật Tài khoản" : "Thêm Tài khoản Mới";
  setValue('cfg-domain', d); setValue('cfg-user', u); setValue('cfg-pass', p);
  const dIn = document.getElementById('cfg-domain'); if (dIn) dIn.readOnly = d !== '';
}
function showCustomConfirm(message) {
  return new Promise((resolve) => {
    const modal = document.getElementById('confirm-modal');
    const msgEl = document.getElementById('confirm-msg');
    const btnYes = document.getElementById('confirm-yes');
    const btnNo = document.getElementById('confirm-no');
    if(!modal || !msgEl) { resolve(confirm(message)); return; }
    msgEl.innerText = message;
    modal.classList.remove('hidden');
    setTimeout(() => { modal.classList.remove('opacity-0'); modal.classList.add('translate-y-4'); }, 10);
    const close = (result) => {
        modal.classList.add('opacity-0'); modal.classList.remove('translate-y-4');
        setTimeout(() => { modal.classList.add('hidden'); resolve(result); }, 300);
    };
    btnYes.onclick = () => close(true); btnNo.onclick = () => close(false);
  });
}
async function syncDataToBrowser() { if (currentMainUrl) { await invoke('refresh_autofill_data', { url: currentMainUrl }).catch(()=>{}); } }

document.addEventListener("DOMContentLoaded", () => {
  if (listen) {
    listen("refresh-accounts", () => { loadPasswordTable(); });
    listen("nsl-ask-save-new", async (e) => {
      const data = e.payload;
      const domainToSave = currentMainUrl.includes('quanlytruonghoc') ? 'hcm.quanlytruonghoc.edu.vn' : 'truong.hcm.edu.vn';
      if (invoke) await invoke('focus_main_window');
      setTimeout(async () => {
          const userAgree = await showCustomConfirm(`Phát hiện tài khoản mới: ${data.user}. Lưu không?`);
          if (userAgree) invoke('save_account', { domain: domainToSave, user: data.user, pass: data.pass }).then(()=>{ loadPasswordTable(); syncDataToBrowser(); });
      }, 200);
    });
    listen("nsl-ask-update", async (e) => {
       const data = e.payload;
       const domainToSave = currentMainUrl.includes('quanlytruonghoc') ? 'hcm.quanlytruonghoc.edu.vn' : 'truong.hcm.edu.vn';
       if (invoke) await invoke('focus_main_window');
       setTimeout(async () => {
           const userAgree = await showCustomConfirm(`Cập nhật mật khẩu cho ${data.user}?`);
           if (userAgree) invoke('save_account', { domain: domainToSave, user: data.user, pass: data.pass }).then(()=>{ loadPasswordTable(); syncDataToBrowser(); });
       }, 200);
    });
  }

  // --- SETUP RESIZE OBSERVER (QUAN TRỌNG) ---
  resizeObserver = new ResizeObserver(() => {
      requestAnimationFrame(() => updateBrowserBounds());
  });
  const mount = document.getElementById('browser-mount-point');
  if (mount) resizeObserver.observe(mount);

  // Toggle Sidebar
  const toggleBtn = document.getElementById("toggle-sidebar");
  if (toggleBtn) {
    toggleBtn.addEventListener("click", () => {
      const sb = document.getElementById("sidebar");
      if (!sb) return;
      const isCollapsed = sb.classList.toggle("sidebar-collapsed");
      document.getElementById("toggle-icon").style.transform = isCollapsed ? "rotate(180deg)" : "rotate(0deg)";
      if (isCollapsed) document.querySelectorAll(".submenu").forEach(s => s.classList.remove("open"));
      
      // Đợi animation CSS chạy xong rồi update bounds
      setTimeout(updateBrowserBounds, 310);
    });
  }

  window.toggleMenu = (menuId, btn) => {
    document.querySelectorAll('.submenu').forEach(el => {
      if (el.id !== menuId) {
        el.classList.remove('open'); el.classList.add('hidden');
        const parent = el.closest('.menu-group')?.querySelector('button');
        if (parent) parent.querySelector('.menu-arrow').style.transform = 'rotate(0deg)';
      }
    });
    const submenu = document.getElementById(menuId);
    if (!submenu) return;
    if (submenu.classList.contains('hidden')) {
      submenu.classList.remove('hidden'); setTimeout(() => submenu.classList.add('open'), 10);
      btn.querySelector('.menu-arrow').style.transform = 'rotate(90deg)';
    } else {
      submenu.classList.remove('open'); submenu.classList.add('hidden');
      btn.querySelector('.menu-arrow').style.transform = 'rotate(0deg)';
    }
  };

  document.getElementById('btn-home')?.addEventListener('click', () => { 
      hideAllViews();
      document.getElementById('view-home').classList.remove('hidden');
      document.getElementById('page-title').innerText = "Trang chủ";
  });
  
  document.getElementById('btn-passwords')?.addEventListener('click', () => { 
      hideAllViews();
      document.getElementById('view-passwords').classList.remove('hidden');
      document.getElementById('view-passwords').classList.add('flex');
      document.getElementById('page-title').innerText = "Quản lý Mật khẩu";
      loadPasswordTable(); 
  });

  // --- HÀM XỬ LÝ CHUNG CHO MENU ---
  const openBrowserView = function(url, name) {
      // 1. Ẩn các view khác
      ['view-home', 'view-passwords'].forEach(id => document.getElementById(id).classList.add('hidden'));
      
      // 2. Hiện mount point
      const mount = document.getElementById('browser-mount-point');
      mount.classList.remove('hidden');
      document.getElementById('page-title').innerText = name || 'Hệ thống';
      
      currentMainUrl = url;

      // 3. Tính toán và gọi Rust
      const rect = mount.getBoundingClientRect();
      if (invoke) {
          invoke('open_embedded_browser', {
              url: url,
              x: Math.round(rect.x),
              y: Math.round(rect.y),
              width: Math.round(rect.width),
              height: Math.round(rect.height)
          });
      }
  };

  // Main Menu Click
  document.querySelectorAll('[data-open-system="true"]').forEach(btn => {
    btn.addEventListener('click', function () {
      if (window.toggleMenu) window.toggleMenu(this.dataset.menu, this);
      openBrowserView(this.dataset.url, this.dataset.name);
    });
  });

  // Submenu Click
  document.querySelectorAll('.menu-link[data-nav]').forEach(el => {
    el.addEventListener('click', function () {
      openBrowserView(this.dataset.nav, "Hệ thống");
    });
  });

  document.getElementById('add-account-btn')?.addEventListener('click', () => { openModal('', '', '', false); });
  document.getElementById('cfg-cancel')?.addEventListener('click', () => { document.getElementById('config-modal').classList.add('hidden'); });
  document.getElementById('cfg-save')?.addEventListener('click', () => { saveConfigToRust(); });

  window.switchToHome?.();
});

// ... (loadPasswordTable và saveConfigToRust giữ nguyên như cũ) ...
async function loadPasswordTable() {
    /* Copy logic cũ */
    const tbody = document.getElementById('password-table-body');
    if(!tbody) return;
    tbody.innerHTML = '<tr><td colspan="4" class="text-center text-slate-500 py-4">Đang tải...</td></tr>';
    const accounts = await (invoke ? invoke('get_all_accounts') : Promise.resolve([]));
    tbody.innerHTML = '';
    if (!accounts || accounts.length === 0) { tbody.innerHTML = '<tr><td colspan="4" class="text-center text-slate-500 py-4">Chưa có dữ liệu</td></tr>'; return; }
    accounts.forEach((acc, i) => {
        const tr = document.createElement('tr');
        tr.className = "hover:bg-white/5 border-b border-white/5";
        tr.innerHTML = `<td class="text-center p-3">${i+1}</td><td class="p-3">${acc.domain}</td><td class="text-cyan-300 p-3">${acc.username}</td>
        <td class="flex justify-center gap-2 p-3">
            <button class="p-1.5 bg-slate-700 rounded" data-copy="${acc.domain}|${acc.username}">📋</button>
            <button class="p-1.5 bg-slate-700 rounded" data-delete="${acc.domain}|${acc.username}">🗑️</button>
        </td>`;
        tbody.appendChild(tr);
    });
    tbody.querySelectorAll('button[data-delete]').forEach(b => b.addEventListener('click', async()=>{
        const [d, u] = b.dataset.delete.split('|');
        if(confirm(`Xóa ${u}?`)) { await invoke('delete_account', {domain: d, username: u}); loadPasswordTable(); }
    }));
    tbody.querySelectorAll('button[data-copy]').forEach(b => b.addEventListener('click', async()=>{
        const [d, u] = b.dataset.copy.split('|');
        const det = await invoke('get_full_account_details', {domain: d, username: u});
        if(det) navigator.clipboard.writeText(det[1]);
    }));
}

window.saveConfigToRust = async () => {
    const d=document.getElementById('cfg-domain').value, u=document.getElementById('cfg-user').value, p=document.getElementById('cfg-pass').value;
    if(d&&u&&p) { await invoke('save_account', {domain:d, user:u, pass:p}); document.getElementById('config-modal').classList.add('hidden'); loadPasswordTable(); syncDataToBrowser(); }
};

setTimeout(()=>{ loadPasswordTable(); }, 500);