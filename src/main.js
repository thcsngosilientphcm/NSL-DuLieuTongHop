// src/main.js
// Entry point: import + khởi tạo các module

import { initSidebar } from './core/sidebar.js';
import { initBrowser, updateBrowserBounds } from './core/browser.js';
import { initModals } from './core/modal.js';
import { initHome } from './menus/home.js';
import { initQuanLyTruongHoc } from './menus/quanlytruonghoc.js';
import { initCSDLNganh } from './menus/csdlnganh.js';
import { initTapHuan } from './menus/taphuan.js';
import { initTemis } from './menus/temis.js';
import { initPasswordManager, loadPasswordTable, syncDataToBrowser } from './menus/quanlymatkhau.js';

document.addEventListener("DOMContentLoaded", () => {
  // Core modules
  initSidebar(() => updateBrowserBounds());
  initBrowser();
  initModals(() => { loadPasswordTable(); syncDataToBrowser(); });

  // Menu modules
  initHome();
  initQuanLyTruongHoc();
  initCSDLNganh();
  initTapHuan();
  initTemis();
  initPasswordManager();

  // Default view
  window.switchToHome?.();
});