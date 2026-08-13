use std::path::{Path, PathBuf};
use vietzip_core::{CompressOptions, CompressionLevel};

/// FR-03. `level` từ Dart dạng chuỗi "fast"/"normal"/"ultra"; không nhận diện được thì
/// rơi về `Normal` thay vì lỗi.
fn parse_level(level: Option<&str>) -> CompressionLevel {
    match level {
        Some("fast") => CompressionLevel::Fast,
        Some("ultra") => CompressionLevel::Ultra,
        _ => CompressionLevel::Normal,
    }
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    // Default utilities - feel free to customize
    flutter_rust_bridge::setup_default_user_utils();
}

/// Bản sao của `vietzip_core::EntryInfo` sang kiểu mà flutter_rust_bridge sinh mã Dart
/// được (frb không thể trực tiếp expose struct từ crate ngoài).
pub struct EntryInfo {
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
}

impl From<vietzip_core::EntryInfo> for EntryInfo {
    fn from(entry: vietzip_core::EntryInfo) -> Self {
        EntryInfo {
            name: entry.name,
            size: entry.size,
            is_dir: entry.is_dir,
        }
    }
}

/// FR-01, FR-02, FR-03, FR-05 — định dạng đầu ra suy ra từ đuôi mở rộng của `dest`.
pub fn compress_files(
    sources: Vec<String>,
    dest: String,
    password: Option<String>,
    level: Option<String>,
) -> Result<(), String> {
    let sources: Vec<PathBuf> = sources.into_iter().map(PathBuf::from).collect();
    let options = CompressOptions {
        password,
        level: parse_level(level.as_deref()),
    };
    vietzip_core::compress(&sources, Path::new(&dest), &options).map_err(|e| e.to_string())
}

/// FR-10, FR-11.
pub fn extract_archive(archive: String, dest_dir: String, password: Option<String>) -> Result<(), String> {
    vietzip_core::extract(Path::new(&archive), Path::new(&dest_dir), password.as_deref())
        .map_err(|e| e.to_string())
}

/// FR-12 — xem trước nội dung mà không giải nén toàn bộ.
pub fn list_archive_entries(archive: String, password: Option<String>) -> Result<Vec<EntryInfo>, String> {
    vietzip_core::list_entries(Path::new(&archive), password.as_deref())
        .map(|entries| entries.into_iter().map(EntryInfo::from).collect())
        .map_err(|e| e.to_string())
}

/// FR-16 — Test Archive.
pub fn test_archive_integrity(archive: String, password: Option<String>) -> Result<bool, String> {
    vietzip_core::test_integrity(Path::new(&archive), password.as_deref()).map_err(|e| e.to_string())
}

/// FR-04 — chia 1 file thành nhiều phần theo kích thước MB. Trả về đường dẫn các phần
/// theo thứ tự.
pub fn split_file(file: String, size_mb: u64) -> Result<Vec<String>, String> {
    vietzip_core::split_file(Path::new(&file), size_mb * 1024 * 1024)
        .map(|parts| parts.into_iter().map(|p| p.to_string_lossy().into_owned()).collect())
        .map_err(|e| e.to_string())
}

/// FR-04 — ghép các phần đã chia (bắt đầu từ phần `.001`) lại thành 1 file.
pub fn join_parts(first_part: String, dest: String) -> Result<(), String> {
    vietzip_core::join_parts(Path::new(&first_part), Path::new(&dest)).map_err(|e| e.to_string())
}

/// FR-18/19 — thêm file/thư mục vào 1 file .zip đã có sẵn, không nén lại từ đầu.
pub fn add_archive_entries(
    archive: String,
    sources: Vec<String>,
    password: Option<String>,
    level: Option<String>,
) -> Result<(), String> {
    let sources: Vec<PathBuf> = sources.into_iter().map(PathBuf::from).collect();
    vietzip_core::add_entries(Path::new(&archive), &sources, password.as_deref(), parse_level(level.as_deref()))
        .map_err(|e| e.to_string())
}

/// FR-18/19 — xoá các entry có tên khớp `names` khỏi 1 file .zip.
pub fn remove_archive_entries(archive: String, names: Vec<String>) -> Result<(), String> {
    vietzip_core::remove_entries(Path::new(&archive), &names).map_err(|e| e.to_string())
}
