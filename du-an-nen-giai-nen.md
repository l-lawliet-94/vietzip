# DỰ ÁN: PHẦN MỀM NÉN/GIẢI NÉN ĐA NỀN TẢNG
### (Tương tự WinRAR / 7-Zip — dành cho người dùng Việt Nam)

---

## 1. TỔNG QUAN DỰ ÁN

### 1.1. Mục tiêu
Xây dựng một phần mềm nén và giải nén dữ liệu miễn phí, mã nguồn mở. Chiều **nén (tạo file mới)** chỉ dùng các định dạng và thư viện hoàn toàn không dính bản quyền/patent (khác với WinRAR — vốn có định dạng RAR độc quyền, đòi hỏi license thương mại để tạo file `.rar`); chiều **giải nén** hỗ trợ thêm việc đọc file `.rar` có sẵn (chỉ đọc, không tạo mới) để tương thích với file người dùng đang có. Giao diện **đơn giản, dễ hiểu**, dùng được ngay không cần hướng dẫn, đồng nhất trải nghiệm trên **mọi nền tảng** (Windows, macOS, Linux, Android) dù mỗi nền tảng có giao diện native riêng; hỗ trợ song ngữ Việt–Anh; hiệu năng và tỷ lệ nén cạnh tranh với WinRAR/7-Zip.

### 1.2. Đối tượng người dùng
- Người dùng cá nhân, văn phòng cần nén/giải nén file thông thường.
- Doanh nghiệp cần nén hàng loạt, bảo mật file bằng mật khẩu.
- Lập trình viên cần tích hợp thư viện nén vào ứng dụng khác (qua CLI/SDK).

### 1.3. Điểm khác biệt / giá trị cốt lõi
- **100% miễn phí bản quyền khi tạo file nén**: không dùng bất kỳ thuật toán, định dạng hay thư viện nào yêu cầu mua license để **tạo** file nén (loại bỏ hoàn toàn việc tạo file `.rar` độc quyền — xem FR-02). Riêng chiều **giải nén** hỗ trợ đọc thêm `.rar` bằng thư viện tương thích cho mục đích giải nén — xem FR-14 và mục 7.
- **Giao diện tối giản & nhất quán trên mọi nền tảng**: ít nút bấm, bố cục quen thuộc, người dùng phổ thông dùng được ngay không cần hướng dẫn, cùng một luồng thao tác dù trên Windows, macOS, Linux hay Android.
- **Menu chuột phải gọn gàng**: chỉ hiện 2-3 lựa chọn phổ biến nhất (Nén..., Giải nén tại đây, Giải nén vào thư mục...), tránh menu con rườm rà nhiều tầng như một số phần mềm nén hiện có.
- **Song ngữ Việt – Anh** ngay từ bản đầu tiên, dễ mở rộng thêm ngôn ngữ khác sau này.
- Không quảng cáo ép buộc, không watermark.
- Tích hợp kiểm tra virus cơ bản khi giải nén (tùy chọn).
- Hỗ trợ nén/giải nén file có tên tiếng Việt có dấu, đường dẫn dài, ổ đĩa mạng.

---

## 2. YÊU CẦU CHỨC NĂNG (Functional Requirements)

### 2.1. Nén file/thư mục
- FR-01: Nén một hoặc nhiều file/thư mục thành 1 file lưu trữ.
- FR-02: Hỗ trợ các định dạng nén đầu ra — **chỉ chọn định dạng mở, miễn phí bản quyền hoàn toàn**: `.zip` (Deflate/zlib), `.7z` (LZMA2), `.tar`, `.tar.gz`, `.tar.bz2`, `.tar.zst` (Zstandard). **Không tạo file `.rar`** vì đây là định dạng nén độc quyền của RARLAB, cần license thương mại để tạo/ghi. (Ứng dụng vẫn hỗ trợ **giải nén/đọc** file `.rar` có sẵn — xem FR-14.)
- FR-03: Chọn mức độ nén: Nhanh (Low), Cân bằng (Normal), Tối đa (Ultra).
- FR-04: Chia nhỏ file nén thành nhiều phần (split archive) theo dung lượng tùy chỉnh (VD: 100MB/phần để gửi email, USB).
- FR-05: Đặt mật khẩu bảo vệ + mã hóa AES-256.
- FR-06: Tùy chọn mã hóa luôn cả tên file bên trong (ẩn danh sách file khi chưa nhập mật khẩu).
- FR-07: Tạo file nén tự giải nén (SFX - self extracting `.exe`).
- FR-08: Thêm comment/ghi chú vào file nén.
- FR-09: Nén theo lịch (schedule) — tự động nén thư mục theo giờ/ngày (tính năng nâng cao).

### 2.2. Giải nén
- FR-10: Giải nén 1 hoặc nhiều file lưu trữ cùng lúc (batch extract).
- FR-11: Giải nén vào thư mục chỉ định hoặc thư mục hiện tại/tự tạo theo tên file.
- FR-12: Xem trước nội dung bên trong file nén mà không cần giải nén toàn bộ (preview/browse).
- FR-13: Giải nén chọn lọc (chỉ chọn 1 số file/thư mục con).
- FR-14: Hỗ trợ **giải nén file `.rar`** (RAR4/RAR5, chỉ đọc — **không hỗ trợ tạo hoặc ghi lại file `.rar`**, xem FR-02) bằng thư viện tương thích cho mục đích giải nén (VD: mã nguồn UnRAR miễn phí của RARLAB — chỉ dành cho việc giải nén — hoặc `libarchive` nếu hỗ trợ RAR). Bắt buộc rà soát kỹ điều khoản giấy phép của thư viện được chọn trước khi tích hợp — xem mục 7.
- FR-15: Tự động nhận diện và xử lý file nén nhiều phần định dạng mở (`.zip.001`, `.7z.001`, `.z01`, v.v).
- FR-16: Kiểm tra tính toàn vẹn file nén (Test Archive) trước khi giải nén.
- FR-17: Sửa file nén bị lỗi (Repair Archive) — mức độ hỗ trợ tùy định dạng.

### 2.3. Quản lý file nén (giao diện dạng trình duyệt file)
- FR-18: Duyệt cấu trúc thư mục bên trong file nén như Windows Explorer.
- FR-19: Thêm/xóa/đổi tên file trực tiếp bên trong file nén đã tồn tại (mở/sửa nhanh).
- FR-20: Tìm kiếm file bên trong file nén.
- FR-21: Kéo-thả (drag & drop) file vào/ra khỏi cửa sổ chương trình.

### 2.4. Tích hợp hệ điều hành
- FR-22: Tích hợp vào menu chuột phải (Windows Explorer, macOS Finder, Nautilus/Linux, GNOME Files) — **thiết kế menu tối giản**, chỉ gồm các mục cốt lõi và dễ hiểu ngay từ tên gọi:
  - Khi bấm chuột phải vào file/thư mục thường: **"Nén vào [tên].zip"**, **"Nén..."** (mở hộp thoại tùy chọn nâng cao).
  - Khi bấm chuột phải vào file nén: **"Giải nén tại đây"**, **"Giải nén vào [tên thư mục]\"**, **"Giải nén..."** (chọn nơi lưu).
  - Không lồng nhiều menu con cấp 2, cấp 3 gây rối mắt; các tùy chọn nâng cao (mật khẩu, mức nén...) chuyển vào hộp thoại riêng khi cần.
- FR-23: Đăng ký làm chương trình mặc định mở các định dạng nén.
- FR-24: Hỗ trợ dòng lệnh (CLI) đầy đủ cho automation/script.
- FR-25: Cung cấp API/SDK (thư viện .dll/.so/.dylib) để nhúng vào ứng dụng khác.

### 2.5. Bảo mật & tiện ích bổ sung
- FR-26: Quét virus cơ bản khi giải nén (tích hợp Windows Defender API hoặc engine mã nguồn mở như ClamAV).
- FR-27: Xóa file gốc an toàn sau khi nén (shredding - ghi đè dữ liệu).
- FR-28: Tạo checksum (MD5/SHA-256) cho file nén để kiểm tra toàn vẹn.
- FR-29: Chuyển đổi định dạng nén (VD: từ .zip sang .7z).

### 2.6. Ngôn ngữ & bản địa hóa
- FR-30: Hỗ trợ **song ngữ Việt – Anh** ngay từ bản 1.0. Tự động nhận diện ngôn ngữ hệ điều hành để chọn mặc định (máy Windows/macOS/Linux tiếng Việt → giao diện tiếng Việt; ngược lại → tiếng Anh), người dùng có thể đổi thủ công trong Cài đặt.
- FR-31: Kiến trúc file ngôn ngữ dạng bảng key-value (VD: JSON/YAML) để dễ dàng cộng đồng đóng góp thêm ngôn ngữ khác sau này (không cần sửa code).
- FR-32: Hỗ trợ đầy đủ Unicode cho tên file tiếng Việt (font UTF-8/UTF-16), không bị lỗi ký tự "?" hay "□".

### 2.7. Giao diện đơn giản (Simple UI)
- FR-33: Màn hình chính chỉ gồm: thanh công cụ với các nút lớn dễ nhận biết (Nén, Giải nén, Kiểm tra, Xem), danh sách file dạng bảng, thanh trạng thái.
- FR-34: Không có tùy chọn nâng cao hiển thị mặc định trên màn hình chính — gom vào mục "Tùy chọn thêm" hoặc "Nâng cao" để tránh gây rối cho người dùng phổ thông.
- FR-35: Có chế độ "Nén nhanh 1 chạm" (One-click Compress) — dùng cấu hình mặc định hợp lý, không cần hỏi thêm gì.
- FR-36: Thông báo lỗi/kết quả bằng ngôn ngữ dễ hiểu, không dùng thuật ngữ kỹ thuật khó hiểu với người dùng phổ thông.

---

## 3. YÊU CẦU PHI CHỨC NĂNG (Non-Functional Requirements)

| Mã | Yêu cầu | Chi tiết |
|---|---|---|
| NFR-01 | Hiệu năng | Tốc độ nén/giải nén tương đương hoặc nhanh hơn 7-Zip với cùng thuật toán (Deflate/LZMA2) |
| NFR-02 | Đa nền tảng | Windows 10/11, macOS 12+, Ubuntu/Debian/Fedora, Android 10+ |
| NFR-03 | Kích thước cài đặt | Bản desktop < 50MB, bản mobile < 30MB |
| NFR-04 | Bộ nhớ | Xử lý file nén > 4GB không bị crash, dùng streaming thay vì load toàn bộ vào RAM |
| NFR-05 | Bảo mật | Mã hóa AES-256, không lưu mật khẩu dạng plaintext |
| NFR-06 | Độ ổn định | Tự động phục hồi khi tiến trình bị ngắt giữa chừng (crash recovery) |
| NFR-07 | Khả năng mở rộng | Kiến trúc plugin để thêm định dạng nén mới dễ dàng |
| NFR-08 | Giấy phép | **Chỉ dùng thư viện/thuật toán mã nguồn mở, miễn phí bản quyền hoàn toàn để TẠO file nén** (LZMA SDK - public domain, zlib - zlib license, Zstandard - BSD, BZip2 - BSD-style). Không tích hợp bất kỳ thành phần nào cần mua license để tạo/ghi file nén (loại việc **tạo** RAR ra khỏi phạm vi dự án). Module **giải nén RAR** là ngoại lệ có kiểm soát: chỉ dùng thư viện có giấy phép cho phép rõ ràng việc giải nén (VD: mã nguồn UnRAR miễn phí — chỉ giải nén, không dùng để tạo phần mềm nén RAR cạnh tranh), phải rà soát điều khoản trước khi tích hợp |
| NFR-09 | Khả năng truy cập (Accessibility) | Hỗ trợ đọc màn hình (screen reader), điều hướng bàn phím |
| NFR-10 | Đa luồng | Tận dụng multi-core CPU khi nén file lớn |
| NFR-11 | Đơn giản & trực quan (Usability) | Người dùng phổ thông thao tác được ngay lần đầu mà không cần đọc hướng dẫn; số bước để nén/giải nén cơ bản tối đa 2 cú nhấp chuột |

---

## 4. KIẾN TRÚC HỆ THỐNG ĐỀ XUẤT

```
┌─────────────────────────────────────────────┐
│              GIAO DIỆN NGƯỜI DÙNG             │
│  Desktop (Qt/Electron)  │  Mobile (Flutter)   │
│  Shell Extension        │  CLI                │
└───────────────┬───────────────────────────────┘
                │  gọi qua API nội bộ
┌───────────────▼───────────────────────────────┐
│            LỚP LÕI NÉN/GIẢI NÉN (CORE)         │
│  - Compression Engine (C/C++/Rust)             │
│  - Codec: Deflate, LZMA2, BZip2, Zstd          │
│  - Archive Format Parser: ZIP, 7Z, TAR (mở)    │
│    + RAR Reader (chỉ đọc, module tách biệt)    │
│  - Encryption Module (AES-256, mã hóa tên file)│
│  - Multi-threading & Streaming I/O             │
└───────────────┬───────────────────────────────┘
                │
┌───────────────▼───────────────────────────────┐
│         LỚP HỆ THỐNG / TIỆN ÍCH               │
│  File System Access │ Checksum │ Logging       │
│  Update Manager     │ Virus Scan Hook          │
└─────────────────────────────────────────────────┘
```

**Nguyên tắc thiết kế:**
- Core engine viết bằng **C++ hoặc Rust** để tối ưu tốc độ, biên dịch thành thư viện dùng chung cho mọi nền tảng.
- Giao diện tách biệt hoàn toàn khỏi lõi xử lý (dễ port sang nền tảng mới).
- Dùng kiến trúc plugin cho từng định dạng nén (dễ thêm định dạng mới, dễ bảo trì).

---

## 5. CÔNG NGHỆ ĐỀ XUẤT

| Thành phần | Công nghệ đề xuất | Giấy phép | Lý do |
|---|---|---|---|
| Core engine | **Rust** (an toàn bộ nhớ) hoặc C++ | — | Hiệu năng cao, đa nền tảng |
| Thuật toán nén | LZMA/LZMA2 (7-Zip SDK), Zstandard, Deflate (zlib), BZip2 | Public Domain / BSD / zlib License | Miễn phí hoàn toàn, không ràng buộc bản quyền, đã kiểm chứng rộng rãi |
| Giải nén RAR (chỉ đọc) | UnRAR source (RARLAB) hoặc `libarchive` nếu hỗ trợ RAR | Giấy phép riêng của RARLAB — chỉ cho phép mục đích giải nén | Cho phép đọc file `.rar` người dùng đã có mà không cần tạo/ghi RAR (tránh vi phạm bản quyền định dạng) |
| Giao diện Windows/macOS/Linux | **Qt 6** (bản LGPL) hoặc **Tauri** (Rust + Web frontend, MIT/Apache) | LGPL / MIT | Nhẹ, đa nền tảng, native look, không tốn phí license nếu tuân thủ LGPL |
| Giao diện Mobile | **Flutter** | Một codebase cho Android/iOS |
| Shell Extension Windows | C++ (Windows Explorer Shell Extension API) | Bắt buộc dùng native API |
| Shell Extension macOS | Swift (Finder Sync Extension) | Bắt buộc theo Apple API |
| Shell Extension Linux | Nautilus/Dolphin script actions | Theo từng desktop environment |
| CLI | Rust/C++ build ra binary độc lập | Không phụ thuộc runtime |
| Mã hóa | OpenSSL hoặc RustCrypto (AES-256-CTR/GCM) | Chuẩn công nghiệp |
| Cập nhật tự động | Sparkle (macOS), WinSparkle (Windows) | Thư viện update phổ biến, miễn phí |
| Đóng gói | Inno Setup/MSIX (Windows), DMG (macOS), AppImage/DEB/RPM (Linux) | Chuẩn từng hệ điều hành |

---

## 6. CÁC GIAI ĐOẠN THỰC HIỆN (ROADMAP)

### GIAI ĐOẠN 0: Chuẩn bị (2-3 tuần)
- Nghiên cứu định dạng file mở: ZIP, 7Z, TAR/GZ/BZ2/ZST (đọc spec chính thức, tất cả đều miễn phí bản quyền).
- Rà soát danh sách thư viện dự kiến sử dụng, xác nhận giấy phép từng thư viện (Public Domain/BSD/MIT/zlib/LGPL) — loại bỏ mọi thành phần cần mua license hoặc có ràng buộc pháp lý cho việc **tạo** file nén, **không đưa việc tạo RAR vào phạm vi dự án**. Rà soát riêng điều khoản giấy phép của thư viện dùng để **giải nén RAR** (chỉ đọc) trước khi quyết định tích hợp.
- Lập kế hoạch chi tiết, phân công nhân sự, chọn công nghệ chính thức.
- Thiết kế wireframe/UI/UX theo hướng **tối giản**: ít màn hình, ít nút bấm, luồng thao tác ngắn nhất có thể; phác thảo menu chuột phải rút gọn.

### GIAI ĐOẠN 1: Xây dựng Core Engine (6-10 tuần)
- Module đọc/ghi ZIP (dùng thư viện miniz/zlib — zlib license).
- Module đọc/ghi 7Z (dựa trên 7-Zip SDK — Public Domain).
- Module đọc/ghi TAR/GZ/BZ2/ZST (dùng zlib, bzip2, libzstd — đều BSD/mã nguồn mở tự do).
- Module đọc RAR (chỉ giải nén, không ghi) — thư viện riêng biệt, tách khỏi các module đọc/ghi ở trên để dễ kiểm soát giấy phép.
- Module mã hóa AES-256 (dùng thư viện mã nguồn mở như RustCrypto/OpenSSL).
- Viết unit test cho từng codec (so sánh với file nén mẫu từ 7-Zip/WinZip để đảm bảo tương thích đọc/ghi định dạng chuẩn).
- Benchmark tốc độ nén/giải nén so với 7-Zip.

### GIAI ĐOẠN 2: CLI & API nội bộ (2-3 tuần)
- Xây dựng công cụ dòng lệnh hoàn chỉnh (nén, giải nén, test, list).
- Thiết kế API (FFI) để giao diện gọi vào core engine.
- Viết tài liệu API cho đội frontend.

### GIAI ĐOẠN 3: Giao diện Desktop (8-12 tuần)
- Xây dựng UI chính **tối giản**: cửa sổ quản lý file nén, thanh công cụ với vài nút lớn dễ hiểu (Nén, Giải nén, Kiểm tra, Xem), ẩn tùy chọn nâng cao vào menu phụ.
- Chức năng kéo-thả, xem trước nội dung, chế độ "Nén nhanh 1 chạm".
- Cài đặt (Settings): chọn ngôn ngữ Việt/Anh, mức nén mặc định, liên kết định dạng file.
- Shell extension cho Windows (menu chuột phải **rút gọn**: chỉ 2-3 mục chính, không lồng nhiều tầng menu con).
- Kiểm thử người dùng thật (usability test) với người chưa từng dùng phần mềm nén để đánh giá độ dễ hiểu.
- Đóng gói bản cài đặt cho Windows (thử nghiệm nội bộ - Alpha).

### GIAI ĐOẠN 4: Đa nền tảng hóa (6-8 tuần)
- Port giao diện sang macOS (Finder Sync Extension).
- Port giao diện sang Linux (tích hợp Nautilus/Dolphin/Thunar).
- Kiểm thử tương thích file nén qua lại giữa các hệ điều hành.

### GIAI ĐOẠN 5: Ứng dụng Mobile (6-8 tuần)
- Xây dựng app Flutter cho Android (ưu tiên trước, thị phần lớn tại VN).
- Tích hợp quyền truy cập file (Storage Access Framework).
- Tối ưu UI cho màn hình cảm ứng.
- (Tùy chọn) Bản iOS sau khi Android ổn định.

### GIAI ĐOẠN 6: Bảo mật & tính năng nâng cao (4-6 tuần)
- Tích hợp quét virus khi giải nén.
- Tính năng tạo SFX self-extracting.
- Tính năng sửa file nén hỏng (repair).
- Tính năng chia nhỏ file (split archive).
- Kiểm thử bảo mật (penetration test cơ bản, kiểm tra rò rỉ mật khẩu trong bộ nhớ).

### GIAI ĐOẠN 7: Bản địa hóa & hoàn thiện UX (2-3 tuần)
- Rà soát toàn bộ chuỗi văn bản tiếng Việt **và tiếng Anh** (đảm bảo tự nhiên, không dịch máy, thuật ngữ nhất quán giữa 2 ngôn ngữ).
- Kiểm tra hiển thị file tiếng Việt có dấu trên mọi nền tảng.
- Rà soát lại toàn bộ giao diện một lần nữa theo tiêu chí đơn giản: loại bỏ nút/tùy chọn thừa, gộp các bước không cần thiết.
- Thu thập phản hồi từ nhóm người dùng thử nghiệm (beta testing) trong cộng đồng Việt Nam và người dùng quốc tế (bản tiếng Anh).

### GIAI ĐOẠN 8: Kiểm thử toàn diện (3-4 tuần)
- Test hiệu năng với file lớn (>10GB), số lượng file nhiều (>100,000 file).
- Test tương thích ngược: mở file nén tạo bởi WinRAR/7-Zip/WinZip.
- Test bảo mật, test đa luồng, test trên phần cứng yếu.
- Sửa lỗi (bug fixing) dựa trên báo cáo từ bản Beta.

### GIAI ĐOẠN 9: Phát hành chính thức (1-2 tuần)
- Chuẩn bị website, trang tải xuống, tài liệu hướng dẫn sử dụng tiếng Việt.
- Đóng gói bản cài đặt cho từng nền tảng (MSIX/EXE, DMG, DEB/RPM/AppImage, APK).
- Thiết lập kênh cập nhật tự động.
- Ra mắt phiên bản 1.0.

### GIAI ĐOẠN 10: Bảo trì & phát triển sau phát hành (liên tục)
- Thu thập phản hồi người dùng, sửa lỗi định kỳ.
- Thêm định dạng nén mới (nếu cần).
- Tối ưu hiệu năng liên tục.
- Cân nhắc mô hình kinh doanh: miễn phí hoàn toàn / freemium (tính năng nâng cao trả phí) / mã nguồn mở cộng đồng.

---

## 7. RỦI RO & LƯU Ý QUAN TRỌNG

- **Không tạo file RAR — chỉ hỗ trợ giải nén**: RAR thuộc sở hữu độc quyền của RARLAB/win.rar GmbH; **tạo/ghi** file `.rar` đòi hỏi license thương mại nên dự án **chủ động không tích hợp việc tạo RAR**. Chiều **giải nén** file `.rar` có sẵn được hỗ trợ để tương thích với người dùng (xem FR-14), dùng thư viện có giấy phép cho phép rõ ràng cho mục đích giải nén (VD: mã nguồn UnRAR miễn phí của RARLAB — lưu ý giấy phép này cấm dùng để xây phần mềm nén/tạo RAR cạnh tranh, chỉ được dùng cho việc giải nén). Với sử dụng thương mại, cần rà soát kỹ điều khoản giấy phép trước khi phát hành. Toàn bộ tính năng **nén** chỉ xoay quanh các định dạng mở: ZIP, 7Z, TAR/GZ/BZ2/ZST. Cần truyền thông rõ với người dùng ngay từ đầu rằng ứng dụng **không tạo được** file `.rar`, chỉ giải nén được.
- **Hiệu năng LZMA2**: Cần tối ưu kỹ vì đây là thuật toán nén chính tạo nên chất lượng nén cao (giống 7-Zip).
- **Cân bằng giữa "đơn giản" và "đầy đủ tính năng"**: Cần kỷ luật thiết kế để không dần "phình to" giao diện theo thời gian — mọi tính năng nâng cao mới nên mặc định ẩn, chỉ hiện khi người dùng chủ động bật.
- **Chất lượng bản dịch song ngữ**: Cần người bản ngữ rà soát cả bản tiếng Việt và tiếng Anh, tránh dịch máy khiến giao diện khó hiểu.
- **Thời gian ước tính tổng thể**: khoảng 9-11 tháng cho đội 4-6 người (backend/core, frontend desktop, mobile, QA) để ra bản 1.0 đầy đủ nền tảng — ngắn hơn so với việc phải tự triển khai đầy đủ cả tạo lẫn đọc RAR, vì chỉ cần tích hợp module giải nén (chỉ đọc), không phải xây dựng bộ mã hóa/ghi RAR.

---

## 8. GỢI Ý NHÂN SỰ TỐI THIỂU

| Vai trò | Số lượng | Trách nhiệm chính |
|---|---|---|
| Lập trình viên Core (C++/Rust) | 2 | Codec nén, mã hóa, hiệu năng |
| Lập trình viên Desktop UI | 1-2 | Qt/Tauri, shell extension |
| Lập trình viên Mobile | 1 | Flutter app |
| QA/Tester | 1 | Kiểm thử đa nền tảng |
| UI/UX Designer | 1 (bán thời gian) | Thiết kế giao diện, bản địa hóa |
| Quản lý dự án | 1 | Điều phối, roadmap |

