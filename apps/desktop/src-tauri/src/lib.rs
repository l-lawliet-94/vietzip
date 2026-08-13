//! Cầu nối Tauri command <-> vietzip_core. Không có lớp FFI riêng vì cùng là Rust
//! (khác với Android sẽ cần cargo-ndk + flutter_rust_bridge — xem ke-hoach-mvp.md mục 4).

use std::path::{Path, PathBuf};
use vietzip_core::{CompressOptions, CompressionLevel, EntryInfo};

/// FR-03. `level` đến từ JS dạng chuỗi "fast"/"normal"/"ultra"; `None`/không nhận diện
/// được đều rơi về `Normal` (mặc định) thay vì lỗi — tuỳ chọn nâng cao nên có phương án
/// dự phòng hợp lý chứ không chặn thao tác nén.
fn parse_level(level: Option<&str>) -> CompressionLevel {
    match level {
        Some("fast") => CompressionLevel::Fast,
        Some("ultra") => CompressionLevel::Ultra,
        _ => CompressionLevel::Normal,
    }
}

#[derive(serde::Serialize)]
struct EntryDto {
    name: String,
    size: u64,
    is_dir: bool,
}

impl From<EntryInfo> for EntryDto {
    fn from(entry: EntryInfo) -> Self {
        EntryDto {
            name: entry.name,
            size: entry.size,
            is_dir: entry.is_dir,
        }
    }
}

/// Ý định khởi chạy app từ dòng lệnh — dùng bởi các mục menu chuột phải mới ("Xem nội dung",
/// "Thêm vào archive") trong `crates/shell-menu`: thay vì tự dựng UI xem/thêm-file bên trong
/// chính DLL COM (rủi ro cao hơn nhiều, xem doc comment của crate đó), handler chỉ mở lại
/// chính app Desktop này kèm 1 tham số dòng lệnh, rồi app tự làm phần còn lại bằng UI đã có
/// sẵn (`viewArchive`/luồng thêm file) — không viết UI mới, không nhân đôi logic.
#[derive(serde::Serialize, Clone)]
struct LaunchIntentDto {
    /// "view" (mở xem nội dung ngay) hoặc "add-to" (chọn 1 archive đích rồi thêm `path` vào).
    action: String,
    path: String,
}

fn parse_launch_intent() -> Option<LaunchIntentDto> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--view" => {
                return args.next().map(|path| LaunchIntentDto { action: "view".to_string(), path });
            }
            "--add-to" => {
                return args.next().map(|path| LaunchIntentDto { action: "add-to".to_string(), path });
            }
            _ => {}
        }
    }
    None
}

static LAUNCH_INTENT: std::sync::OnceLock<Option<LaunchIntentDto>> = std::sync::OnceLock::new();
static LAUNCH_INTENT_CONSUMED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Trả về ý định khởi chạy (nếu có) rồi "tiêu thụ" nó — gọi lần 2 trở đi luôn `None`, tránh
/// việc app tự động Xem/Thêm lại lần nữa nếu người dùng chỉ đơn giản focus lại cửa sổ sau đó.
#[tauri::command]
fn get_launch_intent() -> Option<LaunchIntentDto> {
    if LAUNCH_INTENT_CONSUMED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return None;
    }
    LAUNCH_INTENT.get_or_init(parse_launch_intent).clone()
}

/// Lỗi có cấu trúc gửi qua IPC: `kind` là mã ổn định để phía JS tự dịch sang ngôn ngữ
/// hiện tại (FR-30); `message` là thông điệp tiếng Việt từ core, dùng làm phương án dự
/// phòng cho các lỗi chưa có bản dịch riêng (Io, Archive — xem `vietzip_core::Error::kind`).
#[derive(serde::Serialize)]
struct CommandError {
    kind: &'static str,
    message: String,
}

impl From<vietzip_core::Error> for CommandError {
    fn from(err: vietzip_core::Error) -> Self {
        CommandError {
            kind: err.kind(),
            message: err.to_string(),
        }
    }
}

/// FR-01, FR-02, FR-05 — định dạng đầu ra suy ra từ đuôi mở rộng của `dest`.
#[tauri::command]
fn compress_files(
    sources: Vec<String>,
    dest: String,
    password: Option<String>,
    level: Option<String>,
) -> Result<(), CommandError> {
    let sources: Vec<PathBuf> = sources.into_iter().map(PathBuf::from).collect();
    let options = CompressOptions {
        password,
        level: parse_level(level.as_deref()),
    };
    vietzip_core::compress(&sources, Path::new(&dest), &options).map_err(CommandError::from)
}

/// FR-10, FR-11.
#[tauri::command]
fn extract_archive(
    archive: String,
    dest_dir: String,
    password: Option<String>,
) -> Result<(), CommandError> {
    vietzip_core::extract(Path::new(&archive), Path::new(&dest_dir), password.as_deref())
        .map_err(CommandError::from)
}

/// FR-12 — xem trước nội dung mà không giải nén toàn bộ.
#[tauri::command]
fn list_archive_entries(
    archive: String,
    password: Option<String>,
) -> Result<Vec<EntryDto>, CommandError> {
    vietzip_core::list_entries(Path::new(&archive), password.as_deref())
        .map(|entries| entries.into_iter().map(EntryDto::from).collect())
        .map_err(CommandError::from)
}

/// Kéo 1 dòng trong bảng nội dung ra ngoài Explorer (phần "kéo ra" của mục drag-and-drop, xem
/// CLAUDE.md) — giải nén đúng entry được yêu cầu ra 1 thư mục tạm và trả về đường dẫn tuyệt
/// đối để `main.ts` gọi `startDrag` (plugin `tauri-plugin-drag`) với đường dẫn thật đó.
#[tauri::command]
fn extract_entry_for_drag(
    archive: String,
    entry_name: String,
    password: Option<String>,
) -> Result<String, CommandError> {
    vietzip_core::extract_for_drag(Path::new(&archive), &entry_name, password.as_deref())
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(CommandError::from)
}

/// Đa chọn: kéo NHIỀU dòng đã chọn cùng lúc ra Explorer — cùng cơ chế với
/// `extract_entry_for_drag`, gọi `extract_multiple_for_drag` để chỉ giải nén archive 1 lần
/// thay vì lặp lại cho từng entry.
#[tauri::command]
fn extract_entries_for_drag(
    archive: String,
    entry_names: Vec<String>,
    password: Option<String>,
) -> Result<Vec<String>, CommandError> {
    vietzip_core::extract_multiple_for_drag(Path::new(&archive), &entry_names, password.as_deref())
        .map(|paths| paths.into_iter().map(|p| p.to_string_lossy().into_owned()).collect())
        .map_err(CommandError::from)
}

/// FR-16 — Test Archive.
#[tauri::command]
fn test_archive_integrity(archive: String, password: Option<String>) -> Result<bool, CommandError> {
    vietzip_core::test_integrity(Path::new(&archive), password.as_deref()).map_err(CommandError::from)
}

/// FR-04 — chia 1 file (thường là archive vừa nén) thành nhiều phần theo kích thước MB.
/// Trả về đường dẫn các phần theo thứ tự.
#[tauri::command]
fn split_file(file: String, size_mb: u64) -> Result<Vec<String>, CommandError> {
    vietzip_core::split_file(Path::new(&file), size_mb * 1024 * 1024)
        .map(|parts| parts.into_iter().map(|p| p.to_string_lossy().into_owned()).collect())
        .map_err(CommandError::from)
}

/// FR-04 — ghép các phần đã chia (bắt đầu từ phần `.001`) lại thành 1 file.
#[tauri::command]
fn join_parts(first_part: String, dest: String) -> Result<(), CommandError> {
    vietzip_core::join_parts(Path::new(&first_part), Path::new(&dest)).map_err(CommandError::from)
}

/// FR-07 — tạo file tự giải nén (.exe) từ 1 file `.zip`/`.7z` đã nén sẵn.
///
/// Tìm `sfx-stub.exe` cùng thư mục với chính binary Desktop đang chạy. Hoạt động cho cả dev
/// (`npm run tauri dev`, 2 binary cùng nằm trong `target/debug/`) lẫn bản MSI/NSIS đã cài
/// đặt: `sfx-stub.exe` giờ được đóng gói làm resource (`bundle.resources` trong
/// `tauri.conf.json`) đặt cùng `INSTALLDIR` với binary chính, và `tauri.conf.json`'s
/// `beforeBuildCommand` tự build `vietzip-sfx-stub`/`vietzip` (CLI) trước khi đóng gói nên
/// không còn phụ thuộc bước `cargo build --workspace` thủ công trước đó. **Vẫn còn 1 giới
/// hạn đã biết**: cả 2 resource path (`sfx-stub.exe`, `vietzip.exe`) đang trỏ cứng vào
/// `target/debug/...` — build release thật (`tauri build` không có `-- --debug`) cần đổi
/// 2 đường dẫn đó sang `target/release/...` trước, chưa tự động hoá việc chọn profile.
#[tauri::command]
fn create_sfx(archive: String, output: String, run_after_extract: Option<String>) -> Result<(), CommandError> {
    let current_exe = std::env::current_exe().map_err(|e| CommandError {
        kind: "io",
        message: e.to_string(),
    })?;
    let dir = current_exe.parent().ok_or_else(|| CommandError {
        kind: "archive",
        message: "Không xác định được thư mục chứa ứng dụng".to_string(),
    })?;
    let stub = dir.join(if cfg!(windows) { "sfx-stub.exe" } else { "sfx-stub" });
    if !stub.exists() {
        return Err(CommandError {
            kind: "archive",
            message: format!(
                "Không tìm thấy {} — chỉ hoạt động ở bản dev sau khi `cargo build --workspace`",
                stub.display()
            ),
        });
    }
    vietzip_core::write_sfx(&stub, Path::new(&archive), Path::new(&output), run_after_extract.as_deref())
        .map_err(CommandError::from)
}

/// FR-18/19 — thêm file/thư mục vào 1 file `.zip` đã có sẵn.
#[tauri::command]
fn add_archive_entries(
    archive: String,
    sources: Vec<String>,
    password: Option<String>,
    level: Option<String>,
) -> Result<(), CommandError> {
    let sources: Vec<PathBuf> = sources.into_iter().map(PathBuf::from).collect();
    vietzip_core::add_entries(Path::new(&archive), &sources, password.as_deref(), parse_level(level.as_deref()))
        .map_err(CommandError::from)
}

/// FR-18/19 — xoá các entry có tên khớp `names` khỏi 1 file `.zip`.
#[tauri::command]
fn remove_archive_entries(archive: String, names: Vec<String>) -> Result<(), CommandError> {
    vietzip_core::remove_entries(Path::new(&archive), &names).map_err(CommandError::from)
}

/// FR-18/19 — đổi tên 1 entry trong file `.zip`. Trước đây chỉ có ở CLI (`vietzip rename`);
/// thêm vào Desktop UI theo yêu cầu hoàn thiện các phần còn thiếu.
#[tauri::command]
fn rename_archive_entry(archive: String, old_name: String, new_name: String) -> Result<(), CommandError> {
    vietzip_core::rename_entry(Path::new(&archive), &old_name, &new_name).map_err(CommandError::from)
}

#[derive(serde::Serialize)]
struct AboutInfo {
    name: &'static str,
    version: &'static str,
    license: &'static str,
    third_party_licenses: &'static str,
}

/// Màn hình About — trước đây `LICENSES.md` chỉ được đóng gói vào MSI dưới tên
/// `THIRD-PARTY-LICENSES.md` (xem `bundle.resources`) nhưng không có nơi nào trong UI hiển
/// thị nó cho người dùng. `include_str!` nhúng thẳng nội dung vào binary lúc build — không
/// cần đọc file lúc chạy nên không cần thêm quyền `fs`/`path` nào trong capabilities.
#[tauri::command]
fn get_about_info() -> AboutInfo {
    AboutInfo {
        name: "Vietzip",
        version: env!("CARGO_PKG_VERSION"),
        license: "MIT",
        third_party_licenses: include_str!("../../../../LICENSES.md"),
    }
}

#[derive(serde::Serialize)]
struct BenchmarkResultDto {
    format: String,
    compress_mb_per_sec: f64,
    decompress_mb_per_sec: f64,
    compression_ratio_percent: f64,
}

impl From<vietzip_core::BenchmarkResult> for BenchmarkResultDto {
    fn from(r: vietzip_core::BenchmarkResult) -> Self {
        BenchmarkResultDto {
            format: r.format.to_string(),
            compress_mb_per_sec: r.compress_mb_per_sec,
            decompress_mb_per_sec: r.decompress_mb_per_sec,
            compression_ratio_percent: r.compression_ratio_percent(),
        }
    }
}

/// Tương đương "Tools > Benchmark" của 7-Zip — đo tốc độ nén/giải nén thật trên máy đang
/// chạy, không phải điểm CPU tổng hợp giả lập. Xem `vietzip_core::run_benchmark`.
#[tauri::command]
fn run_benchmark(size_mb: u64, level: Option<String>) -> Result<Vec<BenchmarkResultDto>, CommandError> {
    vietzip_core::run_benchmark(size_mb, parse_level(level.as_deref()))
        .map(|results| results.into_iter().map(BenchmarkResultDto::from).collect())
        .map_err(CommandError::from)
}

#[derive(serde::Serialize)]
struct FileChecksumDto {
    crc32_hex: String,
    sha256_hex: String,
    size_bytes: u64,
}

/// Công cụ CRC/hash độc lập, không liên quan tới thao tác archive nào (tương đương mục
/// "CRC SHA" trong menu chuột phải của 7-Zip). Xem `vietzip_core::compute_checksum`.
#[tauri::command]
fn checksum_file(file: String) -> Result<FileChecksumDto, CommandError> {
    vietzip_core::compute_checksum(Path::new(&file))
        .map(|c| FileChecksumDto {
            crc32_hex: format!("{:08x}", c.crc32),
            sha256_hex: c.sha256_hex,
            size_bytes: c.size_bytes,
        })
        .map_err(CommandError::from)
}

/// Chuyển đổi 1 file nén sang định dạng khác — giải nén rồi nén lại (xem
/// `vietzip_core::convert`, không có cách "chuyển mã trực tiếp" giữa các định dạng archive).
#[tauri::command]
fn convert_archive(
    source: String,
    dest: String,
    source_password: Option<String>,
    dest_password: Option<String>,
    level: Option<String>,
) -> Result<(), CommandError> {
    let options = vietzip_core::ConvertOptions {
        source_password,
        dest_password,
        level: parse_level(level.as_deref()),
    };
    vietzip_core::convert(Path::new(&source), Path::new(&dest), &options).map_err(CommandError::from)
}

#[derive(serde::Serialize)]
struct RepairReportDto {
    recovered: Vec<String>,
    unrecoverable: Vec<String>,
}

impl From<vietzip_core::RepairReport> for RepairReportDto {
    fn from(r: vietzip_core::RepairReport) -> Self {
        RepairReportDto {
            recovered: r.recovered,
            unrecoverable: r.unrecoverable,
        }
    }
}

/// FR-17 — Sửa file nén bị lỗi. Xem `vietzip_core::repair` cho phạm vi hỗ trợ chi tiết theo
/// định dạng (ZIP phục hồi dữ liệu thật, .7z chỉ phát hiện lỗi, định dạng khác không hỗ trợ).
#[tauri::command]
fn repair_archive(archive: String, dest: String, password: Option<String>) -> Result<RepairReportDto, CommandError> {
    vietzip_core::repair(Path::new(&archive), Path::new(&dest), password.as_deref())
        .map(RepairReportDto::from)
        .map_err(CommandError::from)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_drag::init())
        .invoke_handler(tauri::generate_handler![
            compress_files,
            extract_archive,
            list_archive_entries,
            extract_entry_for_drag,
            extract_entries_for_drag,
            get_launch_intent,
            test_archive_integrity,
            split_file,
            join_parts,
            create_sfx,
            add_archive_entries,
            remove_archive_entries,
            rename_archive_entry,
            get_about_info,
            run_benchmark,
            checksum_file,
            convert_archive,
            repair_archive,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
