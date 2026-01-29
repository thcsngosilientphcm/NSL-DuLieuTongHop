import tkinter as tk
from tkinter import messagebox, scrolledtext
import subprocess
import sys
import datetime
import os
import json
import re
import threading

# Đảm bảo script chạy tại thư mục gốc dự án
os.chdir(os.path.dirname(os.path.abspath(__file__)))

class NSLAutoPushApp:
    def __init__(self, root):
        self.root = root
        self.root.title("NSL - GitHub Automation Tool")
        self.root.geometry("600x550")
        self.root.resizable(False, False)
        
        # --- UI ELEMENTS ---
        
        # Header
        lbl_title = tk.Label(root, text="QUẢN LÝ CẬP NHẬT DỰ ÁN", font=("Arial", 16, "bold"), fg="#2c3e50")
        lbl_title.pack(pady=10)

        # Frame chứa lựa chọn
        frame_options = tk.LabelFrame(root, text="Chọn chế độ", font=("Arial", 10, "bold"), padx=10, pady=10)
        frame_options.pack(fill="x", padx=20, pady=5)

        self.mode_var = tk.StringVar(value="fix") # Mặc định chọn sửa lỗi

        # Radio 1: Sửa lỗi
        self.rb_fix = tk.Radiobutton(frame_options, text="Cập nhật chỉnh sửa (Fix Bug)", 
                                     variable=self.mode_var, value="fix", font=("Arial", 11),
                                     command=self.update_ui_state)
        self.rb_fix.pack(anchor="w", pady=5)
        lbl_fix_desc = tk.Label(frame_options, text="   👉 Xóa tag cũ, tạo lại tag cũ để GitHub build lại.", fg="gray", font=("Arial", 9, "italic"))
        lbl_fix_desc.pack(anchor="w")

        # Radio 2: Bản mới
        self.rb_new = tk.Radiobutton(frame_options, text="Phát hành bản mới (New Release)", 
                                     variable=self.mode_var, value="new", font=("Arial", 11),
                                     command=self.update_ui_state)
        self.rb_new.pack(anchor="w", pady=5)
        
        # Frame nhập version (chỉ hiện khi chọn New)
        self.frame_ver = tk.Frame(frame_options)
        self.frame_ver.pack(anchor="w", fill="x", padx=20)
        
        tk.Label(self.frame_ver, text="Phiên bản tiếp theo:", font=("Arial", 10)).pack(side="left")
        self.entry_ver = tk.Entry(self.frame_ver, width=10, font=("Arial", 10, "bold"))
        self.entry_ver.pack(side="left", padx=10)
        
        # Nút Chạy
        self.btn_run = tk.Button(root, text="THỰC HIỆN NGAY", bg="#27ae60", fg="white", 
                                 font=("Arial", 12, "bold"), height=2, width=20,
                                 command=self.start_thread)
        self.btn_run.pack(pady=15)

        # Khu vực Log
        tk.Label(root, text="Nhật ký hoạt động:", font=("Arial", 9, "bold")).pack(anchor="w", padx=20)
        self.txt_log = scrolledtext.ScrolledText(root, height=12, state='disabled', font=("Consolas", 9))
        self.txt_log.pack(fill="both", padx=20, pady=(0, 20))

        # Khởi tạo dữ liệu
        self.current_ver = self.get_current_version_from_file()
        self.next_ver = self.increment_version(self.current_ver)
        self.entry_ver.insert(0, self.next_ver)
        self.update_ui_state()
        
        self.log(f"👋 Xin chào! Phiên bản hiện tại trên máy: v{self.current_ver}")

    def log(self, message):
        self.txt_log.config(state='normal')
        self.txt_log.insert(tk.END, f"{message}\n")
        self.txt_log.see(tk.END)
        self.txt_log.config(state='disabled')

    def run_cmd(self, command, ignore_error=False):
        self.log(f"🔹 Run: {command}")
        try:
            # Chạy lệnh hệ thống, hiển thị tiếng Việt utf-8
            process = subprocess.run(command, shell=True, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8')
            return process.stdout.strip()
        except subprocess.CalledProcessError as e:
            if ignore_error:
                self.log(f"⚠️ Cảnh báo (được bỏ qua): {e.stderr}")
            else:
                self.log(f"❌ LỖI: {e.stderr}")
                raise e

    def get_current_version_from_file(self):
        try:
            with open('package.json', 'r', encoding='utf-8') as f:
                data = json.load(f)
                return data.get('version', '0.0.0')
        except:
            return '0.0.0'

    def increment_version(self, ver):
        # Tăng số cuối (Patch version)
        parts = ver.split('.')
        if len(parts) == 3:
            try:
                parts[2] = str(int(parts[2]) + 1)
                return ".".join(parts)
            except:
                pass
        return ver + ".1"

    def update_ui_state(self):
        mode = self.mode_var.get()
        if mode == 'new':
            self.entry_ver.config(state='normal')
            self.btn_run.config(text=f"PHÁT HÀNH v{self.entry_ver.get()}")
        else:
            self.entry_ver.config(state='disabled')
            self.btn_run.config(text=f"SỬA LỖI v{self.current_ver}")

    def update_files(self, new_ver):
        self.log(f"🔄 Đang cập nhật file cấu hình lên v{new_ver}...")
        
        # 1. package.json
        with open('package.json', 'r+', encoding='utf-8') as f:
            data = json.load(f)
            data['version'] = new_ver
            f.seek(0)
            json.dump(data, f, indent=2, ensure_ascii=False)
            f.truncate()
            
        # 2. tauri.conf.json
        tauri_path = os.path.join('src-tauri', 'tauri.conf.json')
        with open(tauri_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
        data['version'] = new_ver
        with open(tauri_path, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
            
        # 3. Cargo.toml
        cargo_path = os.path.join('src-tauri', 'Cargo.toml')
        with open(cargo_path, 'r', encoding='utf-8') as f:
            content = f.read()
        new_content = re.sub(r'^version\s*=\s*".*"', f'version = "{new_ver}"', content, flags=re.MULTILINE)
        with open(cargo_path, 'w', encoding='utf-8') as f:
            f.write(new_content)
            
        self.log("✅ Đã cập nhật xong số phiên bản trong file.")

    def start_thread(self):
        # Chạy logic trong luồng riêng để không đơ giao diện
        self.btn_run.config(state='disabled')
        threading.Thread(target=self.process_automation).start()

    def process_automation(self):
        try:
            mode = self.mode_var.get()
            
            if mode == 'fix':
                version = self.current_ver
                self.log("="*30)
                self.log(f"🚀 BẮT ĐẦU QUY TRÌNH SỬA LỖI (FIX) - v{version}")
                self.log("="*30)
                
                # 1. Push code mới nhất (nếu có sửa code)
                self.run_cmd("git add .")
                status = self.run_cmd("git status --porcelain")
                if status:
                    self.run_cmd(f'git commit -m "Fix bug re-build v{version}"')
                    self.run_cmd("git push origin main")
                else:
                    self.log("ℹ️ Code không đổi, chỉ chạy lại build...")

                # 2. Xóa tag cũ trên Remote (Github)
                self.log("☁️  Đang xóa Tag cũ trên GitHub...")
                self.run_cmd(f"git push --delete origin v{version}", ignore_error=True)

                # 3. Xóa tag cũ trên Local
                self.log("💻 Đang xóa Tag cũ trên máy...")
                self.run_cmd(f"git tag -d v{version}", ignore_error=True)

                # 4. Tạo tag mới và đẩy lên
                self.log(f"🏷️ Tạo lại Tag v{version}...")
                self.run_cmd(f"git tag v{version}")
                self.run_cmd(f"git push origin v{version}")

            elif mode == 'new':
                new_version = self.entry_ver.get()
                self.log("="*30)
                self.log(f"🚀 BẮT ĐẦU PHÁT HÀNH BẢN MỚI - v{new_version}")
                self.log("="*30)

                # 1. Cập nhật số phiên bản vào file
                self.update_files(new_version)
                
                # 2. Git Commit
                self.run_cmd("git add .")
                time_str = datetime.datetime.now().strftime("%H:%M %d/%m/%Y")
                self.run_cmd(f'git commit -m "Release v{new_version}: {time_str}"')
                
                # 3. Git Push Code
                self.run_cmd("git push origin main")
                
                # 4. Git Tag & Push Tag
                self.log(f"🏷️ Tạo Tag v{new_version}...")
                self.run_cmd(f"git tag v{new_version}")
                self.run_cmd(f"git push origin v{new_version}")
                
                # Cập nhật lại biến nội bộ
                self.current_ver = new_version

            self.log("\n✅✅✅ HOÀN TẤT THÀNH CÔNG!")
            messagebox.showinfo("Thông báo", "Đã xử lý xong! Hãy kiểm tra GitHub Actions.")

        except Exception as e:
            self.log(f"\n❌ QUY TRÌNH THẤT BẠI: {e}")
            messagebox.showerror("Lỗi", f"Có lỗi xảy ra: {e}")
        finally:
            self.btn_run.config(state='normal')
            self.update_ui_state()

if __name__ == "__main__":
    root = tk.Tk()
    app = NSLAutoPushApp(root)
    root.mainloop()