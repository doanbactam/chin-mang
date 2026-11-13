# Kiểm tra trên máy người dùng (SPEC §10.4)

Những mục không tự động hóa được. Chạy bản release (`chin-mang.exe` hoặc `cargo run -p chin-mang --release`), ghi kết quả vào cột cuối: `✔`, `✘ + mô tả`, hoặc `—` nếu không áp dụng. Ghi kèm ngày, máy và hệ điều hành.

**Máy / OS / độ phân giải / DPI:** …………  **Ngày:** …………  **Bản build (commit):** …………

| # | Mục | Cách kiểm | Mong đợi | Kết quả |
|---|---|---|---|---|
| 1 | Trong suốt | Mở game trên desktop có hình nền nhiều chi tiết | Chỉ thấy mèo, quái, HUD; phần còn lại của dải nhìn xuyên thấy desktop, không có khung hay viền đen | |
| 2 | Click xuyên (vùng trống) | Bấm vào vùng trống của dải, chỗ có icon desktop hoặc cửa sổ bên dưới | Windows: khi Ghost mode tắt, dải vẫn nhận click (không xuyên) — ghi đúng hành vi thấy được. Không mất focus của app đang gõ | |
| 3 | Click không xuyên (nút HUD) | Bấm túi, cài đặt, catnip, hộp carton | Nút phản hồi; bảng mở ngay trên dải, mép dưới cửa sổ đứng yên | |
| 4 | Không cướp focus | Đang gõ trong editor thì game khởi động hoặc rơi hộp carton | Con trỏ gõ vẫn ở editor | |
| 5 | Mép dưới đúng | Taskbar ở dưới, rồi đổi sang trái, phải, trên; đổi cả chiều cao taskbar | Dải nằm sát ngay trên taskbar (vùng làm việc), không đè lên và không hở | |
| 6 | Mép trên | Cài đặt → Mép: trên | Dải bám mép trên của vùng làm việc; bảng mở xuống dưới | |
| 7 | Đa màn hình | Cài đặt → Màn hình: chọn màn 2 (khác độ phân giải và DPI) | Dải chuyển sang đúng màn, đúng rộng 50/75/100% | |
| 8 | DPI 125% | Windows Display scale 125% | Hệ số tự động làm tròn về số nguyên; pixel sắc, không nhòe; chữ không bị cắt | |
| 9 | DPI 150% | Như trên với 150% | Như trên | |
| 10 | CPU khi treo | Treo 10 phút, không tương tác; Task Manager (Windows) hoặc `ps -o %cpu,rss -p $(pgrep chin-mang)` (Linux) | Trung bình < 1% CPU (máy 4 nhân) | |
| 11 | RAM khi treo | Như trên | < 150 MB | |
| 12 | Cửa sổ bị che / thu nhỏ | Mở cửa sổ khác che dải 1 phút | Dải giảm còn 2 fps; tiến trình game vẫn đủ (so số màn trước và sau) | |
| 13 | Ghost mode (Windows) | Bật Ghost mode | Dải mờ 60%, click xuyên qua toàn dải; rê chuột lên cụm nút HUD bên phải thì bấm được | |
| 14 | Tự khởi động (Windows) | Bật "Tự khởi động cùng Windows", đăng xuất rồi đăng nhập lại | Game tự chạy; `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` có mục; tắt cài đặt thì mục bị xóa | |
| 15 | Luôn trên cùng | Mở app fullscreen/không fullscreen | Dải nằm trên các cửa sổ thường khi bật "luôn trên cùng" | |
| 16 | Âm thanh mặc định | Lần chạy đầu | Âm nhỏ, nhạc tắt (30/0/40) | |
| 17 | Hyprland (Linux) | Thêm window rule trong README, chạy game | Dải nổi, ghim, không viền và không bóng, nằm ở đáy màn hình | |
| 18 | Hyprland `no_focus` | Có rule `no_focus`, bấm nút túi, cài đặt, hộp carton | Nút vẫn bấm được. Nếu không bấm được, ghi phiên bản Hyprland và thử `no_initial_focus` | |
