# Vietzip

A free, open-source archiver (WinRAR/7-Zip alternative) — core engine, CLI,
and a Tauri desktop app, currently Windows-first.

**[English](#english)** | **[Tiếng Việt](#tiếng-việt)**

---

## English

### Supported formats

- **Compress**: `.zip`, `.7z` (both with optional AES-256 password), plus
  single-file `.gz`/`.bz2`/`.zst`/`.xz`.
- **Extract**: everything above, plus `.rar` (read-only), `.tar` and its
  compressed variants (`.tar.gz`/`.tar.bz2`/`.tar.zst`/`.tar.xz`), `.cab`,
  `.cpio`, `.deb`, `.rpm`, `.lzh`/`.lha`, `.ext2`/`.ext3`/`.ext4`, `.arj`,
  NSIS installer `.exe`, `.chm`, and `.udf`.

Also: split/join large files, self-extracting `.exe` (SFX), add/remove/rename
entries inside an existing `.zip` without recompressing, repair damaged
archives, checksums (CRC32/SHA-256), format conversion, and a benchmark
tool.

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- For the desktop app: [Node.js](https://nodejs.org/) 18+ and the
  [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS
  (on Windows: the MSVC build tools + WebView2, both usually already present)

### Build

Clone the repo, then from the repository root:

```bash
# Core engine + CLI
cargo build --release -p vietzip

# Run the whole test suite
cargo test --workspace
```

The CLI binary ends up at `target/release/vietzip` (`vietzip.exe` on
Windows).

For the desktop app:

```bash
cd apps/desktop
npm install          # once
npm run tauri dev    # hot-reloading dev build
npm run tauri build  # release installers (MSI + NSIS on Windows)
```

Release installers land under `target/release/bundle/`.

### Usage

**CLI** — `vietzip <command> --help` for full details on any command:

```bash
# Compress a folder into a password-protected, AES-256-encrypted 7z
vietzip compress ./my-folder -o archive.7z -p "my-password" -l ultra

# Extract an archive
vietzip extract archive.zip -o ./out

# List contents without extracting
vietzip list archive.rar

# Test archive integrity
vietzip test archive.zip

# Split a large file into parts, then rejoin
vietzip split big-file.zip --size-mb 100
vietzip join big-file.zip.001 -o big-file.zip

# Build a self-extracting .exe
vietzip sfx archive.zip -o installer.exe
```

Other commands: `add`, `remove`, `rename` (edit a `.zip` in place),
`repair` (recover a damaged archive), `checksum`, `convert` (between
archive formats), `benchmark`.

**Desktop app** — launch it and use the four main buttons (Compress /
Extract / Test / View), or drag files onto the window. On Windows with the
MSI installer, right-click integration is also available in Explorer
(Compress to .zip, Extract Here, Extract to subfolder, plus a full context
menu via the installed shell extension).

### License

MIT — see [LICENSE](LICENSE). Third-party components (including a
read-only UnRAR dependency for `.rar` extraction, under RARLAB's own
license terms) are documented in [LICENSES.md](LICENSES.md).

---

## Tiếng Việt

### Định dạng hỗ trợ

- **Nén**: `.zip`, `.7z` (cả 2 đều hỗ trợ mật khẩu AES-256 tuỳ chọn), cộng
  thêm nén file đơn `.gz`/`.bz2`/`.zst`/`.xz`.
- **Giải nén**: tất cả định dạng trên, cộng thêm `.rar` (chỉ đọc), `.tar`
  và các biến thể nén (`.tar.gz`/`.tar.bz2`/`.tar.zst`/`.tar.xz`), `.cab`,
  `.cpio`, `.deb`, `.rpm`, `.lzh`/`.lha`, `.ext2`/`.ext3`/`.ext4`, `.arj`,
  installer NSIS `.exe`, `.chm`, và `.udf`.

Ngoài ra: chia/ghép file lớn, tạo file tự giải nén `.exe` (SFX), thêm/xoá/
đổi tên entry bên trong file `.zip` có sẵn mà không cần nén lại từ đầu, sửa
file nén bị lỗi, tính checksum (CRC32/SHA-256), chuyển đổi định dạng, và
công cụ benchmark.

### Yêu cầu trước khi build

- [Rust](https://rustup.rs/) (bản stable)
- Để build ứng dụng desktop: [Node.js](https://nodejs.org/) 18+ và các yêu
  cầu của [Tauri](https://tauri.app/start/prerequisites/) cho hệ điều hành
  đang dùng (trên Windows: bộ công cụ build MSVC + WebView2, thường đã có
  sẵn)

### Build

Clone repo, rồi từ thư mục gốc:

```bash
# Core engine + CLI
cargo build --release -p vietzip

# Chạy toàn bộ test
cargo test --workspace
```

File thực thi CLI nằm ở `target/release/vietzip` (`vietzip.exe` trên
Windows).

Để build ứng dụng desktop:

```bash
cd apps/desktop
npm install          # chỉ cần 1 lần
npm run tauri dev    # bản dev, hot-reload
npm run tauri build  # bản cài đặt release (MSI + NSIS trên Windows)
```

Bản cài đặt release nằm ở `target/release/bundle/`.

### Cách dùng

**CLI** — gõ `vietzip <lệnh> --help` để xem chi tiết đầy đủ của từng lệnh:

```bash
# Nén 1 thư mục thành file 7z có mật khẩu, mã hoá AES-256
vietzip compress ./thu-muc-cua-toi -o archive.7z -p "mat-khau" -l ultra

# Giải nén 1 file
vietzip extract archive.zip -o ./out

# Liệt kê nội dung mà không giải nén
vietzip list archive.rar

# Kiểm tra tính toàn vẹn của file nén
vietzip test archive.zip

# Chia 1 file lớn thành nhiều phần, rồi ghép lại
vietzip split big-file.zip --size-mb 100
vietzip join big-file.zip.001 -o big-file.zip

# Tạo file tự giải nén .exe
vietzip sfx archive.zip -o installer.exe
```

Các lệnh khác: `add`, `remove`, `rename` (sửa trực tiếp file `.zip`),
`repair` (phục hồi file nén bị lỗi), `checksum`, `convert` (chuyển đổi
giữa các định dạng nén), `benchmark`.

**Ứng dụng desktop** — mở app và dùng 4 nút chính (Nén / Giải nén / Kiểm
tra / Xem), hoặc kéo-thả file vào cửa sổ. Trên Windows dùng bản cài MSI,
còn có tích hợp menu chuột phải trong Explorer (Nén thành .zip, Giải nén
tại đây, Giải nén vào thư mục con, cùng menu ngữ cảnh đầy đủ qua shell
extension đã cài).

### Giấy phép

MIT — xem [LICENSE](LICENSE). Các thành phần bên thứ ba (bao gồm phần phụ
thuộc UnRAR chỉ-đọc để giải nén `.rar`, theo điều khoản giấy phép riêng của
RARLAB) được ghi rõ trong [LICENSES.md](LICENSES.md).
