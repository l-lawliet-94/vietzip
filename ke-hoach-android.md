# KẾ HOẠCH THỰC HIỆN: BẢN ANDROID — TỪ A ĐẾN Z

> **Cập nhật thứ tự:** theo [ke-hoach-mvp.md](ke-hoach-mvp.md) mới nhất, **Desktop đa nền tảng (Windows/macOS/Linux) làm MVP trước, Android làm sau** — đảo ngược so với bản kế hoạch trước đây (từng đặt Android lên trước). File này vẫn giữ nguyên giá trị vì là kế hoạch triển khai chi tiết cho nhánh Android, chỉ khác là **thực hiện ở giai đoạn 2, sau khi Desktop MVP đã ổn định**. Vì core engine (Bước B) và CLI đã được xây dựng xong trong giai đoạn Desktop, khi bắt đầu nhánh Android **có thể bỏ qua Bước A và Bước B bên dưới**, bắt đầu thẳng từ **Bước C (cross-compile & FFI)** — rút ngắn timeline Android còn khoảng 9–13 tuần thay vì 13–18 tuần. Các bước A, B dưới đây được giữ lại nguyên văn để tham khảo/đối chiếu, không cần làm lại.
>
> File này là **kế hoạch triển khai chi tiết** cho nhánh Android, dựa trên đặc tả sản phẩm tổng thể ở [du-an-nen-giai-nen.md](du-an-nen-giai-nen.md). Không thay thế roadmap gốc (Giai đoạn 0–10 trong file đó) — chỉ triển khai chi tiết hoá nhánh Android. Mọi ràng buộc về giấy phép, định dạng, và nguyên tắc giao diện tối giản trong file gốc vẫn áp dụng nguyên vẹn ở đây — đặc biệt: **không tạo file `.rar`**, chỉ **giải nén** `.rar` (xem FR-02, FR-14, NFR-08, mục 7 của file gốc).

---

## 0. Mục tiêu & phạm vi bản Android (giai đoạn 2, sau Desktop MVP)

Ra mắt một ứng dụng Android nén/giải nén dùng được thật, giao diện tối giản, song ngữ Việt–Anh, dùng chung core engine đã xây dựng và kiểm chứng ở giai đoạn Desktop MVP (xem [ke-hoach-mvp.md](ke-hoach-mvp.md) mục 6) — để mở rộng tới nhóm người dùng có thị phần Android lớn tại VN (xem mục 1.2, Giai đoạn 5 file gốc) sau khi bản Desktop đã ổn định.

**Trong phạm vi MVP:**
- Nén: `.zip` (Deflate), `.7z` (LZMA2) — 2 định dạng phổ biến nhất, đủ dùng cho đa số người dùng phổ thông.
- Giải nén: `.zip`, `.7z`, `.tar`, `.tar.gz`, `.tar.bz2`, `.tar.zst`, và **`.rar` (chỉ đọc)**.
- Mật khẩu + mã hóa AES-256 khi nén.
- Giao diện tối giản song ngữ Việt/Anh, nút lớn, luồng thao tác tối đa 2 chạm (theo NFR-11).
- Tích hợp "Mở bằng"/"Chia sẻ" (Android Share/Open-with Intent) — tương đương menu chuột phải trên desktop (FR-22 áp dụng dạng mobile).

**Ngoài phạm vi MVP** (để lại cho các đợt sau, không chặn việc ra mắt):
- SFX self-extracting (`.exe` — không có ý nghĩa trên Android).
- Split archive nhiều phần, nén theo lịch (FR-04, FR-09).
- Quét virus khi giải nén (FR-26) — cần đánh giá riêng engine phù hợp mobile.
- Sửa file nén lỗi (Repair — FR-17).
- Bản iOS (giữ nguyên là tuỳ chọn, làm sau khi Android ổn định, như file gốc đã nêu).

---

## 1. Kiến trúc riêng cho nhánh Android

```
┌─────────────────────────────┐
│   Flutter App (Android)     │  Dart UI, tối giản, song ngữ
├─────────────────────────────┤
│   flutter_rust_bridge (FFI) │  Sinh binding Dart <-> Rust tự động
├─────────────────────────────┤
│   Core Engine (Rust)        │  Dùng CHUNG với Desktop/CLI sau này
│   - Codec: Deflate/LZMA2/   │
│     BZip2/Zstd              │
│   - RAR Reader (chỉ đọc)    │
│   - AES-256                 │
│   - Streaming I/O (SAF)     │
└─────────────────────────────┘
```

Nguyên tắc: core engine viết một lần bằng Rust, biên dịch chéo (cross-compile) ra thư viện `.so` cho Android qua `cargo-ndk`, và tái sử dụng gần như nguyên vẹn khi làm bản Desktop sau này (đúng nguyên tắc "core tách biệt khỏi giao diện" ở mục 4 file gốc). Vì vậy công sức bỏ ra ở nhánh Android không bị lãng phí khi chuyển sang Desktop.

---

## 2. Các bước thực hiện (A → Z)

### BƯỚC A — Chuẩn bị môi trường & xác nhận giấy phép (1 tuần)
- Cài đặt: Rust toolchain, Android NDK + `cargo-ndk`, Flutter SDK, Android Studio, `flutter_rust_bridge_codegen`.
- Khởi tạo cấu trúc thư mục: `/core` (Rust crate dùng chung), `/mobile/android` (Flutter project).
- Rà soát giấy phép từng thư viện dự kiến dùng cho MVP: zlib/miniz (Deflate), LZMA SDK (7z), thư viện đọc RAR (UnRAR source hoặc `libarchive`) — xác nhận đúng như NFR-08 và mục 7 file gốc **trước khi** viết code, không để việc rà soát giấy phép trôi về cuối.

### BƯỚC B — Core engine tối thiểu cho MVP (3–4 tuần)
- Viết crate Rust với API tối giản: `compress()`, `extract()`, `list_entries()`, `test_integrity()`, hỗ trợ callback tiến trình (progress) và hủy giữa chừng.
- Module ghi: ZIP (Deflate), 7Z (LZMA2).
- Module đọc: ZIP, 7Z, TAR/GZ/BZ2/ZST, và **RAR (chỉ đọc, module tách biệt về giấy phép — không gộp chung với các module ghi)**.
- Module mã hóa AES-256 (RustCrypto).
- Unit test trên desktop trước (biên dịch native, nhanh hơn Android nhiều) — so sánh với file mẫu từ 7-Zip/WinRAR để đảm bảo đọc đúng.

### BƯỚC C — Cross-compile & FFI cho Android (1–2 tuần)
- Build `.so` cho các kiến trúc: `arm64-v8a`, `armeabi-v7a` (thiết bị cũ còn phổ biến ở VN), `x86_64` (giả lập/testing).
- Sinh binding Dart bằng `flutter_rust_bridge`.
- Viết app "Hello World" gọi thử `compress()`/`extract()` từ Flutter trên 1 file mẫu để xác nhận toàn bộ pipeline chạy được trước khi đầu tư vào UI.

### BƯỚC D — Truy cập file trên Android (1–2 tuần)
- Dùng **Storage Access Framework (SAF) / Scoped Storage** để chọn file & thư mục nguồn/đích — tránh xin quyền `MANAGE_EXTERNAL_STORAGE` nếu có thể, vì Google Play xét duyệt gắt với quyền này.
- Core engine cần nhận **stream/byte buffer** thay vì chỉ đường dẫn file thô, vì Android trả về `content://` URI chứ không phải path hệ thống — đây là điểm khác biệt kỹ thuật quan trọng so với bản Desktop.
- Đăng ký Android Intent filter để app xuất hiện trong menu "Mở bằng"/"Chia sẻ" khi người dùng chạm vào file `.zip/.7z/.rar/.tar...` từ ứng dụng khác (File Manager, Gmail, Zalo...) — đây là tương đương mobile của "menu chuột phải" (FR-22).

### BƯỚC E — Giao diện Flutter tối giản (3–4 tuần)
- Màn hình chính: danh sách file/thư mục, nút lớn Nén/Giải nén/Kiểm tra/Xem (theo đúng bố cục FR-33 của file gốc, chuyển sang layout mobile).
- Chế độ "Nén nhanh 1 chạm" và giải nén 1 chạm.
- Xem trước nội dung bên trong file nén (browse) trước khi giải nén toàn bộ (FR-12).
- Song ngữ Việt/Anh dùng chung 1 bộ file ngôn ngữ JSON/YAML với các nền tảng khác (FR-30, FR-31) — không viết riêng chuỗi văn bản cho mobile.
- Màn hình Cài đặt: ngôn ngữ, mức nén mặc định.

### BƯỚC F — Bảo mật & tiện ích cơ bản (1–2 tuần)
- Đặt mật khẩu + mã hóa AES-256 khi nén (FR-05).
- Tạo checksum kiểm tra toàn vẹn (tương ứng FR-28).
- Thông báo lỗi bằng ngôn ngữ dễ hiểu, không thuật ngữ kỹ thuật (FR-36).

### BƯỚC G — Kiểm thử (2 tuần)
- Test trên nhiều dòng máy Android thật phổ biến tại VN (Samsung, Xiaomi/Redmi, Oppo, Vivo) — hành vi lưu trữ và quản lý tiến trình nền khác nhau giữa các hãng.
- Test file lớn (>1GB) không crash — xác nhận dùng streaming, không load hết vào RAM (tương ứng NFR-04).
- Test tên file tiếng Việt có dấu hiển thị đúng.
- Test giải nén file `.rar`, `.zip`, `.7z` tạo sẵn bởi WinRAR/7-Zip để xác nhận tương thích đọc.
- Phát hành **closed testing** trên Google Play Console cho nhóm nhỏ trước.

### BƯỚC H — Đóng gói & phát hành (1 tuần)
- Build & ký AAB/APK bản release.
- Thiết lập Google Play Console: mô tả song ngữ, ảnh chụp màn hình, chính sách quyền riêng tư (bắt buộc nếu app xin quyền truy cập file).
- Lộ trình phát hành: closed testing → open testing → production.
- Google Play tự lo kênh cập nhật, không cần tự dựng update server riêng cho bản mobile.

### BƯỚC I — Sau phát hành (liên tục)
- Thu thập crash report & phản hồi người dùng, sửa lỗi định kỳ.
- Tối ưu hiệu năng dựa trên dữ liệu thực tế.
- Core engine (Bước B) và trải nghiệm UI/UX/song ngữ đã được kiểm chứng qua cả Desktop lẫn Android — dùng làm nền tảng chung nếu sau này mở rộng sang iOS.

---

## 3. Ước tính thời gian

| Bước | Nội dung | Thời gian | Cần làm nếu Desktop MVP đã xong trước? |
|---|---|---|---|
| A | Chuẩn bị môi trường & giấy phép | 1 tuần | Không — đã làm ở giai đoạn Desktop |
| B | Core engine tối thiểu | 3–4 tuần | Không — đã có sẵn, tái sử dụng |
| C | Cross-compile & FFI | 1–2 tuần | Có |
| D | Truy cập file (SAF, Intent) | 1–2 tuần | Có |
| E | Giao diện Flutter | 3–4 tuần | Có |
| F | Bảo mật & tiện ích cơ bản | 1–2 tuần | Có |
| G | Kiểm thử | 2 tuần | Có |
| H | Đóng gói & phát hành | 1 tuần | Có |
| **Tổng (làm từ đầu, A→H)** | | **~13–18 tuần (~3,5–4,5 tháng)** | Dùng khi Android là nền tảng đầu tiên |
| **Tổng (Desktop MVP đã xong, chỉ C→H)** | | **~9–13 tuần (~2–3 tháng)** | Trường hợp thực tế theo [ke-hoach-mvp.md](ke-hoach-mvp.md) hiện tại |

Bước I (sau phát hành) chạy liên tục, không tính vào mốc ra mắt.

---

## 4. Rủi ro đặc thù khi làm Android trước

- **Scoped Storage/SAF phức tạp hơn thao tác file thông thường**: cần thiết kế core engine nhận stream ngay từ đầu (Bước B/D) thay vì giả định có đường dẫn file trực tiếp — nếu bỏ qua điều này ở Bước B sẽ phải viết lại phần I/O ở Bước D.
- **Chính sách Google Play về quyền truy cập file**: xin quyền rộng (`MANAGE_EXTERNAL_STORAGE`) dễ bị từ chối duyệt hoặc yêu cầu giải trình; nên thiết kế quanh SAF trước, chỉ xin quyền rộng nếu thật sự cần và có lý do rõ ràng.
- **Phân mảnh thiết bị Android tại VN**: nhiều máy cấu hình thấp, RAM hạn chế, và các hãng (đặc biệt Xiaomi/Oppo) có chính sách quản lý tiến trình nền/pin khắt khe có thể giết app đang nén/giải nén file lớn — cần kiểm thử thực tế trên nhiều dòng máy, không chỉ trên emulator.
- **Giấy phép module đọc RAR trên Android**: thư viện UnRAR source là mã C — cần xác nhận build/link được qua NDK và điều khoản giấy phép vẫn áp dụng đúng khi phân phối dưới dạng APK/AAB (đóng gói nhị phân biên dịch sẵn) trước khi phát hành, không chỉ rà soát một lần ở Bước A.
