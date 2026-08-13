# KẾ HOẠCH MVP CHO TỔNG DỰ ÁN

> File này định nghĩa **MVP (Minimum Viable Product) của toàn bộ dự án**. Nó gộp và cắt gọn từ [du-an-nen-giai-nen.md](du-an-nen-giai-nen.md) (đặc tả sản phẩm đầy đủ, FR-01…FR-36). Mục tiêu: xác định **bộ tính năng tối thiểu nhưng dùng thật được**, để có phản hồi người dùng Việt Nam sớm nhất.
>
> **Thứ tự nền tảng (cụ thể hoá): Windows trước tiên → Linux (Ubuntu) → macOS → cuối cùng mới tới Mobile (Android).** Trước đây "Desktop đa nền tảng" được gộp chung thành 1 giai đoạn (build/đóng gói/kiểm thử cả 3 hệ điều hành cùng lúc); nay tách rõ thứ tự từng hệ điều hành vì thực tế triển khai chỉ có 1 máy Windows để phát triển — Windows làm được và kiểm chứng được ngay, còn Linux/macOS cần môi trường build riêng (máy Linux thật hoặc VM, máy Mac thật) nên phải làm tuần tự khi có môi trường, không thể làm song song cả 3 như dự tính ban đầu. Android vẫn đứng sau toàn bộ Desktop (xem ghi chú đầu file [ke-hoach-android.md](ke-hoach-android.md)).

---

## 1. MVP là gì trong dự án này?

MVP = **Core engine (dùng chung mọi nền tảng) + CLI (kiểm chứng core) + App Desktop, làm tuần tự theo đúng thứ tự: Windows → Linux (Ubuntu) → macOS — sau đó mới tới App Android**.

Không phải là "làm ít tính năng hơn trên tất cả các nền tảng" — mà là **làm ít nền tảng hơn tại một thời điểm, nhưng nền tảng đã làm phải hoàn chỉnh, ổn định, dùng thật được**. Cụ thể theo thứ tự:

1. **Windows** — làm và kiểm thử đầy đủ trước tiên (máy phát triển chính là Windows nên đây là nền tảng khả thi nhất để hoàn thiện sớm).
2. **Linux (Ubuntu)** — tiếp theo, khi có môi trường build Linux (máy thật hoặc VM có đủ toolchain GTK/webkit2gtk mà Tauri cần).
3. **macOS** — sau Linux, khi có máy Mac thật để build/ký/kiểm thử (Apple không cho cross-compile/ký từ Windows hay Linux).
4. **Android** — cuối cùng, sau khi cả 3 hệ điều hành Desktop đã ổn định (xem mục 6).

Mobile (iOS) và các tính năng nâng cao ngoài danh sách mục 2 **không nằm trong đợt MVP đầu tiên**, được đẩy sang giai đoạn sau (xem mục 6).

Lý do chọn Desktop-đa-nền-tảng trước Mobile:
- Core engine là nền tảng dùng chung cho mọi thứ khác — bắt buộc phải làm đầu tiên dù chọn nền tảng nào, không đổi dù thứ tự Desktop/Mobile thay đổi.
- CLI là cách rẻ nhất để kiểm chứng core đúng/đủ trước khi đầu tư vào UI (theo đúng thứ tự Giai đoạn 1 → 2 của roadmap gốc).
- Với **Tauri** (Rust + Web frontend), core Rust được gọi **trực tiếp** qua Tauri command — không cần lớp cross-compile/FFI riêng biệt như Android (`cargo-ndk` + `flutter_rust_bridge`), và **một lần build/kiểm thử phần lõi tích hợp UI đã dùng được cho cả Windows/macOS/Linux** cùng lúc. Ít lớp trung gian hơn, ít bề mặt lỗi hơn so với việc bắt đầu ngay ở Mobile.
- Đối tượng "lập trình viên cần tích hợp CLI/SDK" (mục 1.2 file gốc) và nhóm văn phòng/doanh nghiệp (nén hàng loạt, mật khẩu) — vốn là nhóm dùng Desktop nhiều hơn Mobile — được phục vụ sớm hơn.
- Sau khi Desktop MVP ổn định, phần lõi (core) và phần lớn kinh nghiệm UI/UX tối giản + song ngữ đã kiểm chứng thực tế, khiến việc làm Android (mục 6) nhanh hơn nhiều so với làm Android trước — không phải học lại từ đầu về I/O streaming, format compatibility, v.v.

---

## 2. Phạm vi tính năng MVP (trong phạm vi)

Ánh xạ trực tiếp tới các FR/NFR trong [du-an-nen-giai-nen.md](du-an-nen-giai-nen.md):

| Nhóm | Tính năng trong MVP | FR/NFR liên quan |
|---|---|---|
| Nén | Nén file/thư mục thành `.zip` hoặc `.7z`, mức nén mặc định hợp lý (không cần chọn Nhanh/Cân bằng/Tối đa ở MVP) | FR-01, FR-02, FR-03 (rút gọn) |
| Nén | Đặt mật khẩu + mã hóa AES-256 | FR-05 |
| Giải nén | Giải nén `.zip`, `.7z`, `.tar`, `.tar.gz`, `.tar.bz2`, `.tar.zst` | FR-10, FR-11 |
| Giải nén | **Giải nén `.rar` (chỉ đọc, không tạo mới)** | FR-14 |
| Giải nén | Xem trước nội dung, giải nén chọn lọc | FR-12, FR-13 |
| Kiểm tra | Test Archive (kiểm tra toàn vẹn) | FR-16 |
| Tích hợp | CLI cơ bản (nén, giải nén, test, list) — chủ yếu để kiểm chứng core, không cần đầy đủ automation | FR-24 (rút gọn) |
| Tích hợp | Đăng ký làm chương trình mặc định mở định dạng nén (double-click mở file) | FR-23 |
| Ngôn ngữ | Song ngữ Việt–Anh đầy đủ, Unicode tên file tiếng Việt | FR-30, FR-31, FR-32 |
| Giao diện | Màn hình chính tối giản, nút lớn, ẩn tùy chọn nâng cao, chế độ 1 chạm, thông báo lỗi dễ hiểu | FR-33, FR-34, FR-35, FR-36 |
| Phi chức năng | Streaming I/O cho file lớn (>1GB không crash), đa luồng, đúng tên file tiếng Việt có dấu | NFR-04, NFR-10 |

**Lưu ý:** so với bản MVP trước (ưu tiên Android), FR-23 (đăng ký làm app mặc định) được **đưa vào** phạm vi vì trên Desktop đây là cách chính để mở nhanh 1 file nén (không có "Mở bằng"/Share Intent như Android); còn FR-22 (tích hợp menu chuột phải/shell extension) **vẫn để ngoài phạm vi MVP** — xem mục 3.

---

## 3. Ngoài phạm vi MVP (cố tình để lại)

Các FR sau **không** làm ở MVP — không phải vì khó, mà vì không cần thiết để kiểm chứng "sản phẩm có dùng được không":

- FR-04 (split archive nhiều phần), FR-06 (mã hóa tên file ẩn danh sách), FR-07 (SFX self-extracting), FR-08 (comment/ghi chú), FR-09 (nén theo lịch).
- FR-15 (tự động nhận diện file nén nhiều phần), FR-17 (sửa file nén lỗi).
- FR-18–FR-21 (duyệt/sửa/tìm kiếm/kéo-thả bên trong file nén như trình duyệt file) — MVP chỉ cần xem trước + giải nén chọn lọc (FR-12, FR-13) là đủ để dùng thật, chưa cần thao tác chỉnh sửa archive tại chỗ.
- **FR-22 (tích hợp menu chuột phải/Shell Extension Windows Explorer, Finder, Nautilus...)** — đây là hạng mục nặng nhất bị hoãn: mỗi hệ điều hành cần một cơ chế native riêng (COM DLL cho Windows, Finder Sync Extension cho macOS, script action cho Nautilus/Dolphin trên Linux), tốn công sức không tỷ lệ thuận với giá trị kiểm chứng MVP. App GUI mở trực tiếp + double-click mở file (FR-23) đã đủ để người dùng thử nghiệm nén/giải nén thật.
- FR-25 (đóng gói thành SDK công khai cho lập trình viên khác).
- FR-26 (quét virus), FR-27 (xóa file gốc an toàn/shredding), FR-28 (checksum riêng biệt), FR-29 (chuyển đổi định dạng).
- Toàn bộ Mobile (Android, iOS) — xem mục 6 để biết thứ tự làm sau MVP Desktop.

Nếu trong lúc làm MVP phát sinh nhu cầu thực sự cần một mục ở trên (ví dụ người dùng thử nghiệm không giải nén được file `.zip.001` nhiều phần), đưa ra quyết định rõ ràng để bổ sung — không âm thầm mở rộng phạm vi.

---

## 4. Trình tự & mốc thời gian (Core → CLI → Desktop, tuần tự Windows → Linux → macOS)

Giai đoạn 1–8 dùng chung cho cả 3 hệ điều hành (core/CLI/UI Tauri không phân biệt OS lúc code, chỉ khác lúc đóng gói):

| Giai đoạn | Nội dung | Thời gian |
|---|---|---|
| 1 | Chuẩn bị môi trường, rà soát giấy phép thư viện (kể cả RAR reader) | 1 tuần |
| 2 | Core engine tối thiểu (ZIP/7Z ghi; ZIP/7Z/TAR-family/RAR đọc; AES-256) | 3–4 tuần |
| 3 | CLI cơ bản để kiểm chứng core (nén, giải nén, test, list) | 1 tuần |
| 4 | Setup Tauri (Rust + Web frontend), kết nối trực tiếp core Rust qua Tauri command — không cần lớp FFI riêng vì cùng ngôn ngữ Rust | 1 tuần |
| 5 | Truy cập file hệ thống qua file dialog chuẩn OS (đơn giản hơn Android vì không có Scoped Storage) | 0,5–1 tuần |
| 6 | Giao diện Tauri tối giản, song ngữ Việt/Anh (màn hình chính, nút lớn, chế độ 1 chạm) | 3–4 tuần |
| 7 | Mật khẩu + AES-256 trong UI, checksum test archive, thông báo lỗi thân thiện | 1 tuần |
| 8 | Đăng ký làm app mặc định mở định dạng nén (FR-23) | 0,5–1 tuần |

Từ giai đoạn 9 trở đi, tách riêng theo từng hệ điều hành, làm **tuần tự** (không song song) vì mỗi hệ cần môi trường build/ký riêng:

| Giai đoạn | Hệ điều hành | Nội dung | Trạng thái |
|---|---|---|---|
| 9a | **Windows** | Đóng gói MSI + NSIS, tích hợp menu chuột phải Explorer (FR-22, MSI only) | **Đã xong** — cả 2 bộ cài đã build và kiểm thử thật |
| 10a | **Windows** | Kiểm thử thật trên Windows | **Đã xong** |
| 9b | **Linux (Ubuntu)** | Đóng gói .deb (+ AppImage), tích hợp Nautilus (FR-22) | Code tích hợp Nautilus đã viết sẵn, **chưa build/test được** — cần môi trường Linux thật (máy thật hoặc VM có GTK/webkit2gtk) |
| 10b | **Linux (Ubuntu)** | Kiểm thử thật trên Ubuntu | Chưa làm |
| 9c | **macOS** | Đóng gói DMG, tích hợp Automator Quick Action (FR-22) | Code tích hợp Automator đã viết sẵn (chưa xác minh được trên máy Mac thật), **chưa build/test được** — cần máy Mac thật (Apple không cho ký/cross-compile từ Windows/Linux) |
| 10c | **macOS** | Kiểm thử thật trên macOS | Chưa làm |
| 11 | — | Phát hành: trang tải xuống + GitHub Releases (không có store tập trung như Google Play, phải tự làm trang tải + kênh cập nhật) | Chưa làm — cần cả 3 hệ điều hành xong trước |
| **Tổng** | | | **~16–20 tuần (~4–5 tháng)** nếu tính cả 3 hệ điều hành, nhưng thực tế phụ thuộc vào lúc nào có môi trường Linux/macOS thật, không chỉ thời gian code |

CLI (giai đoạn 3) có thể chạy song song một phần với việc setup Tauri (giai đoạn 4) vì cùng dùng chung core API.

---

## 5. Tiêu chí hoàn thành MVP (Definition of Done)

Vì các hệ điều hành làm **tuần tự** (mục 4), DoD được kiểm theo 2 mức: **DoD từng hệ điều hành** (đủ để coi hệ đó "xong") và **DoD toàn bộ Desktop MVP** (cần cả 3 hệ đều đạt).

### DoD cho mỗi hệ điều hành (áp dụng riêng khi đến lượt hệ đó)

1. Có bản cài đặt tải được cho hệ điều hành đó.
2. Nén/giải nén `.zip` và `.7z` không làm hỏng dữ liệu — kiểm chứng bằng checksum trước/sau trên bộ file test.
3. Giải nén đúng file `.rar` được tạo thật bởi WinRAR (không chỉ file mẫu tự tạo).
4. Không crash với file/thư mục lớn (>1GB) trên cấu hình máy tầm trung.
5. Tên file tiếng Việt có dấu hiển thị và xử lý đúng 100% trong toàn bộ luồng nén/giải nén.
6. Giao diện song ngữ Việt/Anh hoạt động đầy đủ, không sót chuỗi chưa dịch.
7. Người dùng phổ thông (chưa dùng phần mềm nén bao giờ) thao tác nén/giải nén cơ bản được trong ≤ 2 thao tác chuột, không cần hướng dẫn (đúng NFR-11).
8. Double-click vào file nén mở đúng ứng dụng (FR-23).

**Windows: đạt DoD 1–8** — bản cài MSI/NSIS đã build, RAR test bằng file WinRAR thật, file lớn >1GB đã test qua cả ZIP/7Z, tên tiếng Việt có dấu đã test, song ngữ VI/EN đã kiểm tra, FR-23 đã xác minh qua cấu hình WiX. **Chỉ còn thiếu tiêu chí 9 (beta tester thật)** — chưa có người dùng thật ngoài đội dự án dùng thử.

**Linux, macOS: chưa đạt** — do chưa build/kiểm thử được (xem mục 4), không phải do thiếu code.

### DoD cho toàn bộ Desktop MVP (cần thêm, sau khi cả 3 hệ đạt DoD riêng)

9. Đã thu thập phản hồi từ ít nhất một nhóm beta tester thật (không chỉ nội bộ đội dự án) trên ít nhất 1 trong 3 hệ điều hành.
10. Cả 3 hệ điều hành (Windows, Linux, macOS) đều đạt DoD riêng ở trên.

---

## 6. Sau MVP: đến lượt Mobile (Android)

Android đứng **cuối cùng** trong thứ tự nền tảng (mục 1) — chỉ bắt đầu dồn lực đầy đủ sau khi **cả 3 hệ điều hành Desktop (Windows → Linux → macOS, mục 4)** đều đạt DoD riêng. Trong thực tế, một số việc chuẩn bị Android (cross-compile core, dựng khung Flutter) có thể làm sớm hơn khi có thời gian rảnh giữa các giai đoạn Desktop — không bắt buộc chờ tuyệt đối tuần tự 100%, miễn không làm chậm tiến độ Linux/macOS.

Khi MVP Desktop đạt các tiêu chí ở mục 5 và ổn định:
- Core engine đã có sẵn (giai đoạn 2 ở mục 4) được **tái sử dụng** cho Android — không viết lại phần nén/giải nén, chỉ cần cross-compile (`cargo-ndk`) và bọc FFI (`flutter_rust_bridge`) như mô tả trong [ke-hoach-android.md](ke-hoach-android.md).
- Vì core + CLI đã xong ở giai đoạn Desktop, khi bắt đầu làm Android có thể **bỏ qua Bước A và Bước B** của `ke-hoach-android.md` (chuẩn bị môi trường core + viết core engine) — bắt đầu thẳng từ **Bước C (cross-compile & FFI)**, rút ngắn timeline Android còn khoảng **9–13 tuần** thay vì 13–18 tuần như bản kế hoạch cũ.
- Kinh nghiệm UI/UX tối giản + bản dịch song ngữ từ Desktop được tái sử dụng cho Flutter UI, không phải thiết kế lại từ đầu.
- Bản iOS vẫn giữ nguyên là tuỳ chọn, làm sau khi cả Desktop và Android đã ổn định.
- Các FR bị để lại ở mục 3 (đặc biệt FR-22 shell extension) được bổ sung dần theo mức độ ưu tiên thực tế từ phản hồi người dùng MVP, có thể làm trước hoặc sau Android tuỳ nhu cầu thực tế — không cố định thứ tự.

---

## 7. Nhân sự tối thiểu cho MVP

Rút gọn từ mục 8 file gốc — MVP không cần đủ 4-6 người ngay từ đầu:

| Vai trò | Số lượng | Ghi chú |
|---|---|---|
| Lập trình viên Core (Rust) | 1–2 | Kiêm luôn CLI |
| Lập trình viên Desktop UI (Tauri/Web frontend) | 1–2 | Kiêm đóng gói cài đặt 3 hệ điều hành |
| QA/Tester | 1 (bán thời gian đến giai đoạn 10 mới cần toàn thời gian) | Test trên cả 3 hệ điều hành |
| UI/UX & bản địa hóa | 1 (bán thời gian) | Giao diện tối giản + song ngữ |

Không cần lập trình viên Mobile hay Shell Extension riêng ở quy mô MVP — vai trò quản lý dự án có thể do một trong các thành viên trên kiêm nhiệm.
