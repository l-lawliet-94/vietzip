# FR-22 (macOS) — tích hợp Finder qua Automator Quick Actions

Tương đương macOS của "Compress to .zip"/"Extract Here"/"Extract to subfolder" đã làm cho
Windows (`../windows/shell-context-menu.wxs`) và Linux (`../linux/vietzip-nautilus.py`).

## Vì sao dùng Automator Quick Action, không phải Finder Sync Extension

Cách "đúng chuẩn" nhất mà các app macOS thật sự dùng (kể cả 7-Zip/Keka trên Mac) là viết
1 **Finder Sync Extension** — nhưng việc đó cần biên dịch bằng Xcode, ký code, và một loạt
API Swift/Objective-C. Máy dùng để phát triển hiện tại là Windows, **không build được bất
kỳ thứ gì cho macOS** (không có Xcode, không cross-compile được target macOS) — xem
CLAUDE.md mục "Repository state". Automator Quick Action (file `.workflow` — thực chất là
1 bundle chứa file plist XML mô tả 1 bước "Run Shell Script" gọi ra CLI `vietzip`) là lựa
chọn khả thi duy nhất viết được mà **không cần compile**, đổi lại nhãn menu không thể hiện
tên file động như 7-Zip thật (giống hệt đánh đổi đã ghi trong `shell-context-menu.wxs`
cho Windows).

## Mức độ tin cậy — ĐỌC TRƯỚC KHI DÙNG

Khác với phần Windows (đã build + đọc trực tiếp bảng Registry trong file .msi để xác nhận)
và phần Linux (dùng đúng cơ chế `bundle.linux.deb.files`/`postInstallScript` đã đọc trong
mã nguồn thật của `tauri-bundler`), **3 file `.workflow` trong `quick-actions/` chỉ được
xác nhận là XML hợp lệ (well-formed)** — đã tự kiểm tra bằng `[xml]` parser. Cấu trúc plist
bên trong (đúng key, đúng kiểu dữ liệu Automator yêu cầu) viết dựa trên hiểu biết về định
dạng `.workflow` của Apple, KHÔNG được xác minh bằng cách mở thật trong Automator/Finder vì
không có máy Mac. Có khả năng thực (dù không cao) là 1 trong 3 file không mở được hoặc mở
được nhưng Automator hiển thị sai.

**Phương án dự phòng chắc chắn đúng 100%** nếu file có sẵn không hoạt động: tự tạo lại bằng
Automator (mỗi cái mất khoảng 2 phút):
1. Mở **Automator** → New Document → **Quick Action**.
2. "Workflow receives current" chọn **files or folders**, "in" chọn **Finder**.
3. Kéo action **"Run Shell Script"** vào, Shell chọn `/bin/bash`, "Pass input" chọn
   **as arguments**.
4. Dán đúng nội dung shell script tương ứng (xem bảng dưới).
5. File → Save, đặt tên đúng như tên Quick Action mong muốn.

| Quick Action | Shell script |
|---|---|
| Compress to ZIP | `VIETZIP="$(command -v vietzip \|\| echo /usr/local/bin/vietzip)"`<br>`"$VIETZIP" compress "$@"` |
| Extract Here | `VIETZIP="$(command -v vietzip \|\| echo /usr/local/bin/vietzip)"`<br>`for f in "$@"; do "$VIETZIP" extract "$f" --here; done` |
| Extract to Subfolder | `VIETZIP="$(command -v vietzip \|\| echo /usr/local/bin/vietzip)"`<br>`for f in "$@"; do "$VIETZIP" extract "$f" --to-subfolder; done` |

## Cài đặt (nếu file có sẵn hoạt động)

```sh
./install-quick-actions.sh
```

Copy 3 thư mục trong `quick-actions/` vào `~/Library/Services/`. Cần có sẵn CLI `vietzip`
trong PATH (`/usr/local/bin/vietzip` hoặc `/opt/homebrew/bin/vietzip`) — **việc build CLI
cho macOS cũng chưa làm** (cùng lý do không có máy Mac), đây là bước còn thiếu tiếp theo
khi có môi trường macOS thật.

## Known gaps

- Chưa build được `vietzip` CLI cho macOS — Quick Actions này vô dụng cho tới khi có CLI
  binary thật trên máy Mac.
- Không có bước cài đặt tự động khi cài app (Tauri không có post-install hook cho DMG/.app
  trên macOS, khác với `postInstallScript` của .deb trên Linux) — người dùng phải tự chạy
  `install-quick-actions.sh` hoặc làm theo Automator thủ công.
- Nhãn menu cố định, không hiện tên file động (như đã nêu ở trên).
