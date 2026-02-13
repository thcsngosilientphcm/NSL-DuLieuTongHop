import subprocess
import os
import time

def kill_port_5173():
    print("🧹 Cleaning port 5173...")
    try:
        # Lấy danh sách tiến trình đang chiếm port 5173
        # Sử dụng netstat -ano để lấy PID
        result = subprocess.check_output("netstat -ano | findstr :5173", shell=True).decode()
        
        lines = result.strip().split('\n')
        killed_pids = set()

        for line in lines:
            parts = line.split()
            # Định dạng netstat: Proto Local Address Foreign Address State PID
            # Chúng ta cần PID (thường là cột cuối cùng)
            if len(parts) > 4:
                pid = parts[-1]
                
                # Bỏ qua PID 0 (System) và các PID đã kill rồi
                if pid != "0" and pid not in killed_pids:
                    print(f"   -> Killing PID: {pid}")
                    os.system(f"taskkill /PID {pid} /F >nul 2>&1") # >nul để ẩn output rác
                    killed_pids.add(pid)
        
        if not killed_pids:
            print("   -> Port 5173 is clean.")
            
    except subprocess.CalledProcessError:
        # findstr trả về lỗi nếu không tìm thấy gì -> nghĩa là port đang trống
        print("   -> Port 5173 is clean.")
    except Exception as e:
        print(f"   Warning: Could not clean port: {e}")

if __name__ == "__main__":
    # BƯỚC 1: Dọn dẹp port TRƯỚC khi chạy (Quan trọng nhất)
    kill_port_5173()

    try:
        print("🚀 Running Tauri Dev...")
        # BƯỚC 2: Chạy lệnh Tauri
        proc = subprocess.Popen(
            ["npm", "run", "tauri", "dev"],
            shell=True
        )
        proc.wait()

    except KeyboardInterrupt:
        print("\n🛑 Stopping dev server...")
        # Gửi tín hiệu tắt cho tiến trình con
        proc.terminate()

    finally:
        # BƯỚC 3: Dọn dẹp lại lần nữa khi thoát
        print("\n🧹 Final cleanup...")
        kill_port_5173()
        print("✅ Done.")