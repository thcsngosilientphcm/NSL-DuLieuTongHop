// src-tauri/src/scripts.rs

pub fn get_autofill_script(accounts_json: &str) -> String {
    let template = r##"
        (function(){
            // Chặn Web tự ý đổi layout
            try {
                window.moveTo = function(){}; window.resizeTo = function(){};
                window.moveBy = function(){}; window.resizeBy = function(){};
            } catch(e) {}

            var SAVED_ACCOUNTS = __NSL_ACCOUNTS__;

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
                    if (!el || el.tagName !== 'INPUT') return false;
                    var id = el.id || '';
                    return id.endsWith('tbU') || el.name.indexOf('$tbU') !== -1 || el.placeholder === 'Tên đăng nhập';
                }
                function removeDropdown() { if (currentDropdown) { currentDropdown.remove(); currentDropdown = null; } }

                document.addEventListener('click', function(e) {
                    if (isUserInput(e.target)) createDropdown(e.target);
                    else if (currentDropdown && !currentDropdown.contains(e.target)) removeDropdown();
                }, true);

                function createDropdown(targetInput) {
                    if (!SAVED_ACCOUNTS || SAVED_ACCOUNTS.length === 0) return;
                    if (currentDropdown && currentDropdown.dataset.target === targetInput.id) return;
                    removeDropdown();
                    var rect = targetInput.getBoundingClientRect();
                    var div = document.createElement('div');
                    div.style = 'position:absolute; background:white; border:1px solid #0078d4; z-index:2147483647; box-shadow:0 4px 12px rgba(0,0,0,0.2); overflow-y:auto; max-height:200px; border-radius:4px; font-family: sans-serif; min-width: 200px;';
                    div.style.left = (rect.left + window.scrollX) + 'px';
                    div.style.top = (rect.bottom + window.scrollY + 2) + 'px';
                    div.style.width = Math.max(rect.width, 250) + 'px';
                    div.dataset.target = targetInput.id;

                    SAVED_ACCOUNTS.forEach(function(acc) {
                        var item = document.createElement('div');
                        item.innerHTML = '<div style="font-weight:bold; font-size:13px; margin-bottom:2px;">' + acc.username + '</div><div style="font-size:11px; color:#666;">********</div>';
                        item.style = 'padding:8px 12px; cursor:pointer; border-bottom:1px solid #eee; color:#333;';
                        item.onmouseenter = function() { this.style.backgroundColor = '#eef6ff'; };
                        item.onmouseleave = function() { this.style.backgroundColor = 'white'; };
                        item.onmousedown = function(e) {
                            e.preventDefault();
                            targetInput.value = acc.username;
                            targetInput.dispatchEvent(new Event('input', { bubbles: true }));
                            
                            var passInput = null;
                            if (targetInput.id) passInput = document.getElementById(targetInput.id.replace('tbU', 'tbP'));
                            if (!passInput) passInput = document.querySelector('input[name*="$tbP"]');
                            if (!passInput) passInput = document.querySelector('input[type="password"]');

                            if (passInput) {
                                passInput.value = acc.password;
                                passInput.dispatchEvent(new Event('input', { bubbles: true }));
                            }
                            removeDropdown();
                        };
                        div.appendChild(item);
                    });
                    document.body.appendChild(div);
                    currentDropdown = div;
                }
            }

            // --- 3. LOGIN MONITOR (CACHE & TRIGGER) ---
            function setupLoginMonitor() {
                // A. Cache dữ liệu khi nhập
                function syncCache() {
                    var userIn = document.getElementById('ContentPlaceHolder1_tbU') || document.querySelector('input[name*="$tbU"]');
                    var passIn = document.getElementById('ContentPlaceHolder1_tbP') || document.querySelector('input[name*="$tbP"]');
                    if (userIn && passIn && userIn.value && passIn.value) {
                        var data = encodeURIComponent(JSON.stringify({user: userIn.value, pass: passIn.value}));
                        window.location.hash = "NSL_DATA|" + Date.now() + "|" + data;
                    }
                }
                document.addEventListener('input', syncCache, true);
                document.addEventListener('change', syncCache, true);

                // B. Kích hoạt khi bấm nút (Chỉ đánh dấu là "đang đăng nhập")
                function triggerLogin() {
                    syncCache(); // Lưu lần cuối
                    setTimeout(function(){ window.location.hash = "NSL_TRIGGER|" + Date.now(); }, 50);
                }

                // Hook các kiểu bấm nút
                if (typeof window.WebForm_DoPostBackWithOptions !== 'undefined') {
                    var original = window.WebForm_DoPostBackWithOptions;
                    window.WebForm_DoPostBackWithOptions = function(o) { triggerLogin(); return original(o); };
                }
                var btn = document.getElementById('ContentPlaceHolder1_btOK');
                if(btn) btn.addEventListener('mousedown', triggerLogin);
                
                document.addEventListener('keydown', function(e) {
                    if (e.key === 'Enter') {
                        var el = e.target;
                        if (el.tagName === 'INPUT' && (el.type === 'password' || el.id.includes('tbU'))) triggerLogin();
                    }
                }, true);
            }

            if (document.readyState === 'loading') {
                document.addEventListener('DOMContentLoaded', function() { setupAutofill(); setupLoginMonitor(); });
            } else {
                setupAutofill(); setupLoginMonitor();
            }
        })();
    "##;
    
    template.replace("__NSL_ACCOUNTS__", accounts_json)
}