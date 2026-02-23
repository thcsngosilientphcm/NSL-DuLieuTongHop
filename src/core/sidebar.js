// src/core/sidebar.js
// Logic sidebar: toggle, menu expand/collapse

const SIDEBAR_WIDTH_OPEN = 310.0;
const SIDEBAR_WIDTH_COLLAPSED = 64.0;

export function initSidebar(onBoundsUpdate) {
    const toggleBtn = document.getElementById("toggle-sidebar");
    if (toggleBtn) {
        toggleBtn.addEventListener("click", () => {
            const sb = document.getElementById("sidebar");
            if (!sb) return;
            const isCollapsed = sb.classList.toggle("sidebar-collapsed");
            document.getElementById("toggle-icon").style.transform = isCollapsed ? "rotate(180deg)" : "rotate(0deg)";
            if (isCollapsed) document.querySelectorAll(".submenu").forEach(s => s.classList.remove("open"));

            // Đợi animation CSS chạy xong rồi update bounds
            setTimeout(() => { if (onBoundsUpdate) onBoundsUpdate(); }, 310);
        });
    }

    // Toggle submenu
    window.toggleMenu = (menuId, btn) => {
        document.querySelectorAll('.submenu').forEach(el => {
            if (el.id !== menuId) {
                el.classList.remove('open'); el.classList.add('hidden');
                const parent = el.closest('.menu-group')?.querySelector('button');
                if (parent) {
                    const arrow = parent.querySelector('.menu-arrow');
                    if (arrow) arrow.style.transform = 'rotate(0deg)';
                }
            }
        });
        const submenu = document.getElementById(menuId);
        if (!submenu) return;
        if (submenu.classList.contains('hidden')) {
            submenu.classList.remove('hidden'); setTimeout(() => submenu.classList.add('open'), 10);
            const arrow = btn.querySelector('.menu-arrow');
            if (arrow) arrow.style.transform = 'rotate(90deg)';
        } else {
            submenu.classList.remove('open'); submenu.classList.add('hidden');
            const arrow = btn.querySelector('.menu-arrow');
            if (arrow) arrow.style.transform = 'rotate(0deg)';
        }
    };
}
