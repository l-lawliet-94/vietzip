//! NSIS installer (`.exe` được tạo bởi Nullsoft Scriptable Install System) — chỉ giải nén, mở
//! rộng theo yêu cầu phủ danh sách định dạng 7-Zip hỗ trợ (xem CLAUDE.md mục "Broader
//! extract-only format coverage").
//!
//! Dùng crate `nsis` (Apache-2.0, pure Rust, tác giả ATRAPS LLC) — dependency cực sạch
//! (`goblin`/`flate2`/`lzma-rs`, 2 cái sau đã có sẵn trong dự án), không FFI. Ban đầu được
//! xây cho phân tích mã độc/reverse engineering nên có kỷ luật phòng thủ code rất cao
//! (`deny(unwrap_used, expect_used, panic, indexing_slicing, arithmetic_side_effects,
//! unsafe_code)`) — phù hợp cho việc parse file nhị phân không tin cậy.
//!
//! **Doc comment của crate ở API `ExtractedFile::decompress` nói sai** ("not yet supported
//! for solid installers, it returns an error") — đã tự kiểm chứng bằng test thật với 1
//! installer solid-mode thật (`tests/data/nsis_full_featured.exe`, nén `/SOLID lzma`):
//! `decompress()` thực ra hoạt động đúng cho cả solid lẫn non-solid (đọc thẳng
//! `NsisInstaller::solid_data()` — dữ liệu đã giải nén sẵn khi parse xong, không phải để dành
//! decode sau). Không sửa lại doc của crate (không phải code của dự án này), chỉ ghi lại ở
//! đây để lần sau không bị đánh lừa bởi chính doc của nó.
//!
//! **Giới hạn thật, có chủ đích**:
//! - `NsisInstaller::from_bytes` nhận `&[u8]`, không phải `Read` — phải đọc nguyên file vào
//!   RAM trước khi parse (khác nguyên tắc streaming cho file lớn áp dụng ở nơi khác, vd
//!   `rpm_format.rs`/`ext_format.rs`). Chấp nhận được: installer NSIS thực tế hiếm khi vượt
//!   quá vài trăm MB, và bản thân API crate được thiết kế zero-copy dựa trên có sẵn cả buffer.
//! - `.files()` chỉ lộ ra TÊN NGUỒN (tham số của lệnh `File` trong script `.nsi`), không lộ
//!   đường dẫn đích đầy đủ (crate không diễn giải toàn bộ bytecode `SetOutPath`) — 2 entry có
//!   thể cùng tên nguồn nhưng khác thư mục đích thật trong installer gốc (vd `full_featured.exe`
//!   trong fixture test: "payload.txt" xuất hiện 2 lần, ở 2 section khác nhau). `dedupe_name()`
//!   (dưới) đổi tên khi trùng thay vì ghi đè âm thầm — mất dữ liệu không dấu vết còn tệ hơn 1
//!   tên file hơi xấu.
//! - Không có trường "kích thước gốc" riêng trong header entry (độ dài trong prefix là kích
//!   thước đã NÉN) — `list_entries` ở đây phải giải nén thật để biết kích thước chính xác,
//!   khác mọi module khác trong dự án (nơi liệt kê luôn rẻ hơn giải nén). Ghi rõ ở đây, không
//!   giả vờ đây là thao tác nhẹ.

use crate::{EntryInfo, Error, Result};
use nsis::NsisInstaller;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

fn map_err(err: nsis::error::Error) -> Error {
    Error::Archive(err.to_string())
}

/// Dò xem `path` có phải 1 installer NSIS thật không — thử parse thật bằng
/// `NsisInstaller::from_bytes` (crate không có API "chỉ dò chữ ký, không parse" riêng), dùng
/// cho `Format::from_path` khi gặp đuôi `.exe` (installer NSIS không có đuôi mở rộng riêng để
/// nhận diện qua tên file, xem `lib.rs::Format::from_path`). Bất kỳ lỗi nào (file không tồn
/// tại, không đọc được, không phải NSIS) đều trả `false`, không phải lỗi cứng — nhất quán với
/// hợp đồng của `Format::from_path` (trả `None` khi không nhận diện được, không panic).
pub fn sniff(path: &Path) -> bool {
    let Ok(data) = fs::read(path) else { return false };
    NsisInstaller::from_bytes(&data).is_ok()
}

fn relative_path(raw_name: &str) -> PathBuf {
    Path::new(&raw_name.replace('\\', "/"))
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .collect()
}

/// Đổi tên khi trùng (xem doc module) thay vì ghi đè âm thầm — `payload.txt` trùng thứ 2 trở
/// thành `payload_2.txt`, thứ 3 thành `payload_3.txt`, v.v.
fn dedupe_name(used: &mut HashSet<PathBuf>, path: PathBuf) -> PathBuf {
    if used.insert(path.clone()) {
        return path;
    }
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned());
    let parent = path.parent().map(PathBuf::from).unwrap_or_default();
    let mut n = 2u32;
    loop {
        let candidate_name = match &ext {
            Some(e) => format!("{stem}_{n}.{e}"),
            None => format!("{stem}_{n}"),
        };
        let candidate = parent.join(candidate_name);
        if used.insert(candidate.clone()) {
            return candidate;
        }
        n += 1;
    }
}

fn load(archive_path: &Path) -> Result<Vec<u8>> {
    fs::read(archive_path).map_err(|e| Error::io(archive_path, e))
}

pub fn extract(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let data = load(archive_path)?;
    let installer = NsisInstaller::from_bytes(&data).map_err(map_err)?;
    fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;

    let mut used_names: HashSet<PathBuf> = HashSet::new();
    for file in installer.files() {
        let file = file.map_err(map_err)?;
        let raw_name = file.name().map_err(map_err)?.to_string();
        let rel = relative_path(&raw_name);
        if rel.as_os_str().is_empty() {
            continue;
        }
        let rel = dedupe_name(&mut used_names, rel);
        let out_path = dest_dir.join(&rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let content = file.decompress().map_err(map_err)?;
        let mut out_file = File::create(&out_path).map_err(|e| Error::io(&out_path, e))?;
        out_file.write_all(&content).map_err(|e| Error::io(&out_path, e))?;
    }
    Ok(())
}

pub fn list_entries(archive_path: &Path) -> Result<Vec<EntryInfo>> {
    let data = load(archive_path)?;
    let installer = NsisInstaller::from_bytes(&data).map_err(map_err)?;

    let mut used_names: HashSet<PathBuf> = HashSet::new();
    let mut entries = Vec::new();
    for file in installer.files() {
        let file = file.map_err(map_err)?;
        let raw_name = file.name().map_err(map_err)?.to_string();
        let rel = relative_path(&raw_name);
        if rel.as_os_str().is_empty() {
            continue;
        }
        let rel = dedupe_name(&mut used_names, rel);
        // Không có kích thước gốc trong header — phải giải nén thật để biết, xem doc module.
        let content = file.decompress().map_err(map_err)?;
        entries.push(EntryInfo {
            name: rel.to_string_lossy().replace('\\', "/"),
            size: content.len() as u64,
            is_dir: false,
        });
    }
    Ok(entries)
}

pub fn test_integrity(archive_path: &Path) -> Result<bool> {
    let data = load(archive_path)?;
    let installer = NsisInstaller::from_bytes(&data).map_err(map_err)?;
    for file in installer.files() {
        let file = file.map_err(map_err)?;
        file.decompress().map_err(map_err)?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture installer NSIS THẬT, tải từ chính repo GitHub gốc của crate `nsis`
    /// (`ATRAPSLLC/nsis-rs`, `tests/fixtures/*.exe` — build thật bằng NSIS compiler thật, kèm
    /// script `.nsi` gốc để biết chính xác nội dung, không phải đoán), cùng giấy phép
    /// Apache-2.0 với crate. `deflate_nonsolid.exe` chứa đúng 2 file: "payload.txt" (nội dung
    /// "This is a test payload for NSIS fixture generation.") và "config.ini". Provenance đầy
    /// đủ trong `crates/core/tests/data/README.md`.
    #[test]
    fn sniff_recognizes_real_nsis_installer() {
        let path = Path::new("tests/data/nsis_deflate_nonsolid.exe");
        assert!(sniff(path));
    }

    #[test]
    fn sniff_rejects_non_nsis_exe() {
        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp.path().join("not-nsis.exe");
        fs::write(&fake, b"day khong phai la installer NSIS that su").unwrap();
        assert!(!sniff(&fake));
        assert!(!sniff(Path::new("does/not/exist.exe")));
    }

    #[test]
    fn list_extract_and_test_roundtrip_on_real_nonsolid_fixture() {
        let archive = Path::new("tests/data/nsis_deflate_nonsolid.exe");
        assert!(test_integrity(archive).unwrap());

        let entries = list_entries(archive).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.name == "payload.txt"));
        assert!(entries.iter().any(|e| e.name == "config.ini"));

        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("out");
        extract(archive, &dest).unwrap();
        let content = fs::read_to_string(dest.join("payload.txt")).unwrap();
        assert!(content.contains("This is a test payload for NSIS fixture generation"));
    }

    /// `full_featured.exe` nén `/SOLID lzma` — xác nhận solid mode hoạt động đúng (xem doc
    /// module) và xác nhận `dedupe_name` không ghi đè khi 2 entry cùng tên nguồn "payload.txt"
    /// (script gốc có 2 section đều `File "payload.txt"`, xem `.nsi` trong provenance).
    #[test]
    fn extract_handles_solid_mode_and_duplicate_source_names() {
        let archive = Path::new("tests/data/nsis_full_featured.exe");
        assert!(test_integrity(archive).unwrap());

        let entries = list_entries(archive).unwrap();
        assert_eq!(entries.len(), 3, "phai co 3 entry: payload.txt, config.ini, payload_2.txt");
        assert!(entries.iter().any(|e| e.name == "payload.txt"));
        assert!(entries.iter().any(|e| e.name == "payload_2.txt"), "entry trung ten phai duoc doi ten: {entries:?}");
        assert!(entries.iter().any(|e| e.name == "config.ini"));

        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("out");
        extract(archive, &dest).unwrap();
        assert!(dest.join("payload.txt").exists());
        assert!(dest.join("payload_2.txt").exists(), "khong duoc ghi de am tham");
        let a = fs::read(dest.join("payload.txt")).unwrap();
        let b = fs::read(dest.join("payload_2.txt")).unwrap();
        assert_eq!(a, b, "2 entry cung ten nguon phai cung noi dung trong fixture nay");
    }
}
