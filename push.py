import subprocess
import sys
import datetime
import os

# Tự động chuyển hướng về đúng thư mục chứa file script để tránh lỗi path
os.chdir(os.path.dirname(os.path.abspath(__file__)))

def run_cmd(command):
    print(f"🔹 Đang chạy: {command}")
    try:
        # Sử dụng encoding utf-8 để hiển thị tiếng Việt nếu có
        result = subprocess.run(command, shell=True, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8')
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        print(f"❌ LỖI: {e.stderr}")
        sys.exit(1)

def main():
    print("="*40)
    print("🚀 NSL-DuLieuTongHop: AUTO PUSH SYSTEM")
    print("="*40)

    # 1. Add files
    run_cmd("git add .")

    # 2. Tạo commit message tự động theo giờ
    time_str = datetime.datetime.now().strftime("%H:%M %d/%m/%Y")
    commit_msg = f"Auto update NSL Data: {time_str}"
    
    # 3. Commit
    status = run_cmd("git status --porcelain")
    if status:
        run_cmd(f'git commit -m "{commit_msg}"')
    else:
        print("ℹ️ Không có file mới cần đóng gói, sẽ kiểm tra việc đẩy code cũ...")

    # 4. Push (Luôn luôn chạy lệnh này)
    print("☁️  Đang đẩy lên GitHub...")
    run_cmd("git push origin main")

if __name__ == "__main__":
    main()