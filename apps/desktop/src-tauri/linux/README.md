# FR-22 (Linux) — tích hợp Nautilus (GNOME Files)

Tương đương Linux của "Compress to .zip"/"Extract Here"/"Extract to subfolder" — xem
`../windows/shell-context-menu.wxs` (Windows) và `../macos/README.md` (macOS).

## Cách hoạt động

- `vietzip-nautilus.py` — extension Python cho Nautilus, dùng `Nautilus.MenuProvider` để
  thêm 3 mục chuột phải, gọi ra CLI `vietzip` qua `subprocess`.
- Đóng gói vào bản `.deb` qua `bundle.linux.deb` trong `tauri.conf.json`:
  - `files`: copy CLI vào `/usr/bin/vietzip`, copy extension vào
    `/usr/share/nautilus-python/extensions/vietzip-nautilus.py`.
  - `recommends: ["python3-nautilus"]` — gợi ý cài, không bắt buộc (`depends` cứng), vì
    không phải ai cài Vietzip cũng dùng GNOME Files.
  - `postInstallScript: postinst.sh` — chạy `nautilus -q` sau khi cài để Nautilus nạp lại
    extension mới.

Cơ chế `files`/`postInstallScript` được xác nhận có thật bằng cách đọc mã nguồn đã vendor
của `tauri-bundler` (`bundle/linux/debian.rs`, `utils/fs_utils.rs::copy_custom_files`) —
**đúng luôn cả chiều key/value** (khác với `bundle.resources` ở gốc `tauri.conf.json`: ở đó
key=nguồn/value=đích, còn ở `deb.files` thì ngược lại, key=đích/value=nguồn — dễ nhầm nếu
không đọc code thật, đã kiểm tra kỹ trước khi viết).

## CHƯA kiểm thử trên Linux thật

Máy dev hiện tại là Windows, không build/chạy được Linux (xem CLAUDE.md mục "Đóng gói
Desktop cho Linux"). Điểm chưa chắc chắn lớn nhất trong `vietzip-nautilus.py`: **chữ ký
phương thức `get_file_items`/`get_background_items` đổi giữa Nautilus GTK3 cũ (có tham số
`window` đứng đầu) và GTK4 mới (Nautilus 43+, vd Ubuntu 24.04 — bỏ tham số đó)**. Code dùng
`*args` rồi lấy phần tử cuối để tương thích cả 2 phiên bản — đây là kỹ thuật nhiều extension
Nautilus thật ngoài đời dùng cho đúng lý do này, nhưng chưa tự chạy thử để xác nhận trên
máy thật.

## AppImage không có tích hợp này

`bundle.linux.deb` chỉ áp dụng cho gói `.deb` — AppImage là định dạng portable, không có
bước cài đặt hệ thống (không postinst, không ghi vào `/usr/share/...`), nên không có cách
tự nhiên nào để đăng ký Nautilus extension từ 1 AppImage. Giống hệt giới hạn "NSIS không có
context-menu" bên Windows — hạn chế của định dạng đóng gói, không phải thiếu sót.
