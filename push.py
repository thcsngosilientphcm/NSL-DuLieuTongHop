import subprocess
import sys
import datetime
import os
import json
import re

# Đảm bảo script chạy tại thư mục gốc dự án
os.chdir(os.path.dirname(os.path.abspath(__file__)))

def run_cmd(command):
    print(f"🔹 Run: {command}")
    try:
        result = subprocess.run(command, shell=True, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8')
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        print(f"❌ LỖI: {e.stderr}")
        sys.exit(1)

def sync_versions():
    print("🔄 Đang đồng bộ phiên bản từ package.json...")
    
    # 1. Đọc version từ package.json (SẾP)
    try:
        with open('package.json', 'r', encoding='utf-8') as f:
            pkg_data = json.load(f)
            version = pkg_data.get('version')
            if not version:
                print("❌ Không tìm thấy 'version' trong package.json")
                sys.exit(1)
            print(f"📌 Phiên bản hiện tại: {version}")
    except FileNotFoundError:
        print("❌ Không tìm thấy file package.json")
        sys.exit(1)

    # 2. Cập nhật tauri.conf.json
    tauri_path = os.path.join('src-tauri', 'tauri.conf.json')
    try:
        with open(tauri_path, 'r', encoding='utf-8') as f:
            tauri_data = json.load(f)
        
        if tauri_data.get('version') != version:
            tauri_data['version'] = version
            with open(tauri_path, 'w', encoding='utf-8') as f:
                json.dump(tauri_data, f, indent=2, ensure_ascii=False)
            print(f"✅ Đã cập nhật tauri.conf.json -> {version}")
        else:
            print("creating... tauri.conf.json đã khớp.")
            
    except FileNotFoundError:
        print(f"⚠️ Không tìm thấy {tauri_path}")

    # 3. Cập nhật Cargo.toml (Dùng Regex để giữ nguyên comment)
    cargo_path = os.path.join('src-tauri', 'Cargo.toml')
    try:
        with open(cargo_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Tìm dòng version = "..." trong [package] và thay thế
        # Pattern tìm: version = "x.y.z"
        new_content = re.sub(r'^version\s*=\s*".*"', f'version = "{version}"', content, flags=re.MULTILINE)
        
        if content != new_content:
            with open(cargo_path, 'w', encoding='utf-8') as f:
                f.write(new_content)
            print(f"✅ Đã cập nhật Cargo.toml -> {version}")
        else:
             print("creating... Cargo.toml đã khớp.")

    except FileNotFoundError:
        print(f"⚠️ Không tìm thấy {cargo_path}")
    
    return version

def main():
    print("="*40)
    print("🚀 NSL-DuLieuTongHop: AUTO SYNC & PUSH")
    print("="*40)

    # --- BƯỚC 1: ĐỒNG BỘ VERSION ---
    current_version = sync_versions()

    # --- BƯỚC 2: GIT ADD ---
    run_cmd("git add .")

    # --- BƯỚC 3: KIỂM TRA THAY ĐỔI & COMMIT ---
    status = run_cmd("git status --porcelain")
    time_str = datetime.datetime.now().strftime("%H:%M %d/%m/%Y")
    
    if status:
        commit_msg = f"Update v{current_version}: {time_str}"
        run_cmd(f'git commit -m "{commit_msg}"')
        print(f"📦 Đã đóng gói code với version {current_version}")
    else:
        print("ℹ️ Không có thay đổi file, kiểm tra đẩy dữ liệu cũ...")

    # --- BƯỚC 4: PUSH CODE ---
    print("☁️  Đang đẩy code lên GitHub...")
    run_cmd("git push origin main")

    # --- BƯỚC 5: HỎI TẠO TAG RELEASE ---
    print("\n" + "-"*40)
    print(f"❓ Bạn có muốn phát hành bản cài đặt v{current_version} không?")
    choice = input("👉 Nhấn 'y' rồi Enter để phát hành (các phím khác để bỏ qua): ").strip().lower()

    if choice == 'y':
        print(f"🚀 Đang kích hoạt GitHub Actions cho bản v{current_version}...")
        # Xóa tag cũ nếu trùng (để build lại nếu cần)
        try:
            run_cmd(f"git tag -d v{current_version}")
            run_cmd(f"git push --delete origin v{current_version}")
            print("   (Đã xóa tag cũ trùng tên)")
        except:
            pass # Bỏ qua nếu tag chưa tồn tại
            
        run_cmd(f"git tag v{current_version}")
        run_cmd(f"git push origin v{current_version}")
        print(f"\n✅ HOÀN TẤT! Hãy lên GitHub tab Actions để xem quá trình Build.")
    else:
        print("\n✅ Đã đẩy code nhưng KHÔNG tạo bản cài đặt.")

if __name__ == "__main__":
    main()