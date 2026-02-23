// src-tauri/src/scripts.rs

pub fn get_autofill_script() -> String {
    let template = r##"
        (function(){
            // Chặn Web tự ý đổi layout
            try {
                window.moveTo = function(){}; window.resizeTo = function(){};
                window.moveBy = function(){}; window.resizeBy = function(){};
            } catch(e) {}

            var SAVED_ACCOUNTS = [];
            window.__NSL_UPDATE_ACCOUNTS__ = function(newData) {
                SAVED_ACCOUNTS = newData;
            };

            // --- 1. GIAO DIỆN HỎI LƯU MẬT KHẨU (INJECTED UI) ---
            window.nslShowSavePrompt = function(user, pass, isUpdate) {
                // Xóa cái cũ nếu có
                var old = document.getElementById('nsl-save-prompt');
                if(old) old.remove();

                var div = document.createElement('div');
                div.id = 'nsl-save-prompt';
                var title = isUpdate ? 'Cập nhật mật khẩu?' : 'Lưu mật khẩu?';
                var sub = isUpdate ? 'Bạn có muốn cập nhật mật khẩu mới cho:' : 'Bạn có muốn lưu tài khoản này không?';
                
                div.innerHTML = `
                    <div style="display:flex; align-items:start; gap:12px;">
                        <div style="font-size:24px;">🔑</div>
                        <div style="flex:1;">
                            <div style="font-weight:bold; font-size:14px; margin-bottom:4px; color:#1f2937;">${title}</div>
                            <div style="font-size:12px; color:#4b5563; margin-bottom:2px;">${sub}</div>
                            <div style="font-weight:bold; font-size:13px; color:#0891b2; margin-bottom:12px;">${user}</div>
                            <div style="display:flex; gap:8px; justify-content:flex-end;">
                                <button id="nsl-no" style="padding:6px 12px; border:1px solid #d1d5db; background:white; border-radius:6px; font-size:12px; cursor:pointer; color:#374151;">Không</button>
                                <button id="nsl-yes" style="padding:6px 12px; border:none; background:#0891b2; border-radius:6px; font-size:12px; cursor:pointer; color:white; font-weight:bold;">Lưu</button>
                            </div>
                        </div>
                        <button id="nsl-close" style="background:none; border:none; cursor:pointer; font-size:16px; color:#9ca3af; padding:0;">&times;</button>
                    </div>
                `;
                
                // Style cho hộp thoại nổi
                div.style.cssText = 'position:fixed; top:20px; right:20px; width:320px; background:white; padding:16px; border-radius:12px; box-shadow: 0 10px 40px rgba(0,0,0,0.2), 0 0 0 1px rgba(0,0,0,0.05); z-index: 2147483647; font-family: sans-serif; opacity:0; transform:translateY(-20px); transition:all 0.3s ease;';
                
                document.body.appendChild(div);
                
                // Animation hiện ra
                setTimeout(() => { div.style.opacity = '1'; div.style.transform = 'translateY(0)'; }, 50);

                // Xử lý sự kiện
                document.getElementById('nsl-yes').onclick = function() {
                    var data = encodeURIComponent(JSON.stringify({user: user, pass: pass}));
                    window.location.hash = "NSL_CMD_SAVE|" + Date.now() + "|" + data;
                    div.style.opacity = '0'; setTimeout(() => div.remove(), 300);
                };
                document.getElementById('nsl-no').onclick = function() {
                    div.style.opacity = '0'; setTimeout(() => div.remove(), 300);
                };
                document.getElementById('nsl-close').onclick = function() {
                    div.style.opacity = '0'; setTimeout(() => div.remove(), 300);
                };
            };

            // --- 2. AUTOFILL (GIỮ NGUYÊN) ---
            function setupAutofill() {
                var currentDropdown = null;
                function isUserInput(el) {
                    try {
                        if (!el || el.tagName !== 'INPUT') return false;
                        var id = el.id || '';
                        var name = el.name || '';
                        var ph = el.placeholder || '';
                        return id.indexOf('tbU') !== -1 || name.indexOf('$tbU') !== -1 || name === 'lname' || ph === 'Tên đăng nhập' || ph.toLowerCase() === 'tài khoản';
                    } catch(e) { return false; }
                }
                function removeDropdown() { if (currentDropdown) { currentDropdown.remove(); currentDropdown = null; } }

                document.addEventListener('click', function(e) {
                    try {
                        if (isUserInput(e.target)) createDropdown(e.target);
                        else if (currentDropdown && !currentDropdown.contains(e.target)) removeDropdown();
                    } catch(err) {}
                }, true);

                function createDropdown(targetInput) {
                    try {
                        if (!SAVED_ACCOUNTS || SAVED_ACCOUNTS.length === 0) return;
                        var targetKey = targetInput.id || targetInput.name || 'nsl-target';
                        if (currentDropdown && currentDropdown.dataset.target === targetKey) return;
                        removeDropdown();
                        var rect = targetInput.getBoundingClientRect();
                        var div = document.createElement('div');
                        div.style = 'position:absolute; background:white; border:1px solid #0078d4; z-index:2147483647; box-shadow:0 4px 12px rgba(0,0,0,0.2); overflow-y:auto; max-height:200px; border-radius:4px; font-family: sans-serif; min-width: 200px;';
                        div.style.left = (rect.left + window.scrollX) + 'px';
                        div.style.top = (rect.bottom + window.scrollY + 2) + 'px';
                        div.style.width = Math.max(rect.width, 250) + 'px';
                        div.dataset.target = targetKey;

                        SAVED_ACCOUNTS.forEach(function(acc) {
                            var item = document.createElement('div');
                            item.innerHTML = '<div style="font-weight:bold; font-size:13px; margin-bottom:2px;">' + acc.username + '</div><div style="font-size:11px; color:#666;">********</div>';
                            item.style = 'padding:8px 12px; cursor:pointer; border-bottom:1px solid #eee; color:#333;';
                            item.onmouseenter = function() { this.style.backgroundColor = '#eef6ff'; };
                            item.onmouseleave = function() { this.style.backgroundColor = 'white'; };
                            item.onmousedown = function(e) {
                                e.preventDefault();
                                
                                // Dùng native setter để Vue/React nhận biết thay đổi
                                var nativeSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
                                nativeSetter.call(targetInput, acc.username);
                                targetInput.dispatchEvent(new Event('input', { bubbles: true }));
                                targetInput.dispatchEvent(new Event('change', { bubbles: true }));
                                
                                var passInput = null;
                                if (targetInput.id) passInput = document.getElementById(targetInput.id.replace('tbU', 'tbP'));
                                if (!passInput) passInput = document.querySelector('input[name*="$tbP"]');
                                if (!passInput) passInput = document.querySelector('input[name="pass"]');
                                if (!passInput) passInput = document.querySelector('input[type="password"]');

                                if (passInput) {
                                    nativeSetter.call(passInput, acc.password);
                                    passInput.dispatchEvent(new Event('input', { bubbles: true }));
                                    passInput.dispatchEvent(new Event('change', { bubbles: true }));
                                }
                                removeDropdown();
                            };
                            div.appendChild(item);
                        });
                        document.body.appendChild(div);
                        currentDropdown = div;
                    } catch(err) {}
                }
            }

            // --- 3. LOGIN MONITOR (CACHE & TRIGGER) ---
            function setupLoginMonitor() {
                // A. Cache dữ liệu khi nhập
                function syncCache() {
                    try {
                        var userIn = document.getElementById('ContentPlaceHolder1_tbU') || document.querySelector('input[name*="$tbU"]') || document.querySelector('input[name="lname"]');
                        var passIn = document.getElementById('ContentPlaceHolder1_tbP') || document.querySelector('input[name*="$tbP"]') || document.querySelector('input[name="pass"]');
                        if (userIn && passIn && userIn.value && passIn.value) {
                            var data = encodeURIComponent(JSON.stringify({user: userIn.value, pass: passIn.value}));
                            window.location.hash = "NSL_DATA|" + Date.now() + "|" + data;
                        }
                    } catch(err) {}
                }
                document.addEventListener('input', syncCache, true);
                document.addEventListener('change', syncCache, true);

                // B. Kích hoạt khi bấm nút (Chỉ đánh dấu là "đang đăng nhập")
                function triggerLogin() {
                    try {
                        syncCache(); // Lưu lần cuối

                        // Lưu dự phòng vào localStorage để lỡ web chuyển trang quá nhanh, trình duyệt nhúng trang sau vẫn nhớ
                        var u = document.querySelector('input[name*="$tbU"]') || document.querySelector('input[name="lname"]');
                        var p = document.querySelector('input[name*="$tbP"]') || document.querySelector('input[name="pass"]');
                        var userVal = u ? u.value : '';
                        var passVal = p ? p.value : '';

                        if (u && p && userVal) {
                            localStorage.setItem('NSL_PENDING_USER', userVal);
                            localStorage.setItem('NSL_PENDING_PASS', passVal);
                            localStorage.setItem('NSL_PENDING_TIME', Date.now().toString());
                        }

                        var data = encodeURIComponent(JSON.stringify({user: userVal, pass: passVal}));
                        setTimeout(function(){ window.location.hash = "NSL_TRIGGER_DATA|" + Date.now() + "|" + data; }, 50);
                    } catch(err){}
                }

                // Hook các kiểu bấm nút
                try {
                    if (typeof window.WebForm_DoPostBackWithOptions !== 'undefined') {
                        var original = window.WebForm_DoPostBackWithOptions;
                        window.WebForm_DoPostBackWithOptions = function(o) { triggerLogin(); return original(o); };
                    }
                } catch(err){}

                // Hook các kiểu bấm nút tĩnh & động (Event Delegation cho SPA/React/Vue)
                document.addEventListener('mousedown', function(e) {
                    try {
                        var el = e.target;
                        if (!el) return;
                        var isLoginBtn = false;
                        if (el.id === 'ContentPlaceHolder1_btOK') isLoginBtn = true;
                        else if (el.closest && el.closest('.vt-login-form__login-button')) isLoginBtn = true;
                        else if (el.closest && el.closest('button[type="submit"]')) isLoginBtn = true;
                        
                        if (isLoginBtn) {
                            triggerLogin();
                        }
                    } catch(err){}
                }, true);
                
                document.addEventListener('submit', function(e) {
                    try { triggerLogin(); } catch(err){}
                }, true);
                
                document.addEventListener('keydown', function(e) {
                    try {
                        if (e.key === 'Enter') {
                            var el = e.target;
                            if (el && el.tagName === 'INPUT') {
                                var id = el.id || '';
                                var name = el.name || '';
                                if (el.type === 'password' || id.indexOf('tbU') !== -1 || name === 'lname') triggerLogin();
                            }
                        }
                    } catch(err){}
                }, true);
            }

            function requestAccounts() {
                try {
                    setTimeout(function(){ window.location.hash = "NSL_REQ_ACCOUNTS|" + Date.now(); }, 50);
                } catch(err){}
            }

            // Polling liên tục localStorage để bắt kịp SPA navigation (Vue Router đè hash)
            setInterval(function() {
                try {
                    var pu = localStorage.getItem('NSL_PENDING_USER');
                    var pp = localStorage.getItem('NSL_PENDING_PASS');
                    var pt = localStorage.getItem('NSL_PENDING_TIME');
                    if (pu && pp && pt) {
                        var elapsed = Date.now() - parseInt(pt);
                        // Chờ ít nhất 1.5 giây từ lúc bấm đăng nhập để trang đích load xong
                        if (elapsed > 1500 && elapsed < 180000) {
                            localStorage.removeItem('NSL_PENDING_USER');
                            localStorage.removeItem('NSL_PENDING_PASS');
                            localStorage.removeItem('NSL_PENDING_TIME');
                            var data = encodeURIComponent(JSON.stringify({user: pu, pass: pp}));
                            window.location.hash = "NSL_TRIGGER_DATA|" + Date.now() + "|" + data;
                        }
                    }
                } catch(e){}
            }, 1500);

            if (document.readyState === 'loading') {
                document.addEventListener('DOMContentLoaded', function() { setupAutofill(); setupLoginMonitor(); requestAccounts(); });
            } else {
                setupAutofill(); setupLoginMonitor(); requestAccounts();
            }
        })();
    "##;

    template.to_string()
}
