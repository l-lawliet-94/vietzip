//! Microsoft Cabinet (`.cab`) — chỉ giải nén, mở rộng theo yêu cầu người dùng phủ toàn bộ
//! danh sách định dạng 7-Zip hỗ trợ (xem CLAUDE.md mục "7-Zip feature parity").
//!
//! Dùng crate `cab` (MIT, pure Rust, cùng tác giả với `msi`) — hỗ trợ đọc cả 3 kiểu nén
//! CAB thật sự phổ biến (None/MSZIP/LZX), chỉ thiếu Quantum (kiểu nén cũ, hiếm gặp ngoài
//! 1 số bộ cài InstallShield rất cũ) — nếu gặp sẽ trả lỗi rõ ràng từ chính crate, không
//! phải silent-fail.
//!
//! CAB không có khái niệm thư mục thật (không giống ZIP/TAR có entry riêng cho từng thư
//! mục) — tên file có thể chứa `\` làm dấu phân cách, được giữ nguyên trong `EntryInfo.name`
//! để lớp điều hướng thư mục phía Desktop UI (vốn đã tách theo `/`) tự nhận diện. Vì vậy khi
//! giải nén cần tự tạo thư mục cha còn thiếu (không có entry thư mục riêng để dựa vào).

use crate::{EntryInfo, Error, Result};
use std::fs::File;
use std::io;
use std::path::Path;

/// CAB dùng `\` làm dấu phân cách trong tên file nội bộ (kế thừa quy ước Windows) — chuẩn
/// hoá về `/` để khớp cách các định dạng khác trong dự án này trình bày đường dẫn.
fn normalize_name(name: &str) -> String {
    name.replace('\\', "/")
}

fn open_cabinet(archive_path: &Path) -> Result<cab::Cabinet<File>> {
    let file = File::open(archive_path).map_err(|e| Error::io(archive_path, e))?;
    cab::Cabinet::new(file).map_err(|e| Error::io(archive_path, e))
}

fn collect_names(cabinet: &cab::Cabinet<File>) -> Vec<String> {
    let mut names = Vec::new();
    for folder in cabinet.folder_entries() {
        for file in folder.file_entries() {
            names.push(file.name().to_string());
        }
    }
    names
}

pub fn extract(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let mut cabinet = open_cabinet(archive_path)?;
    std::fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;

    for raw_name in collect_names(&cabinet) {
        let normalized = normalize_name(&raw_name);
        let dest_path = dest_dir.join(&normalized);
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let mut reader = cabinet
            .read_file(&raw_name)
            .map_err(|e| Error::io(archive_path, e))?;
        let mut out = File::create(&dest_path).map_err(|e| Error::io(&dest_path, e))?;
        io::copy(&mut reader, &mut out).map_err(|e| Error::io(&dest_path, e))?;
    }
    Ok(())
}

pub fn list_entries(archive_path: &Path) -> Result<Vec<EntryInfo>> {
    let cabinet = open_cabinet(archive_path)?;
    let mut result = Vec::new();
    for folder in cabinet.folder_entries() {
        for file in folder.file_entries() {
            result.push(EntryInfo {
                name: normalize_name(file.name()),
                size: file.uncompressed_size() as u64,
                is_dir: false,
            });
        }
    }
    Ok(result)
}

pub fn test_integrity(archive_path: &Path) -> Result<bool> {
    let mut cabinet = open_cabinet(archive_path)?;
    for raw_name in collect_names(&cabinet) {
        let mut reader = cabinet
            .read_file(&raw_name)
            .map_err(|e| Error::io(archive_path, e))?;
        io::copy(&mut reader, &mut io::sink()).map_err(|e| Error::io(archive_path, e))?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cab::{CabinetBuilder, CompressionType};
    use std::io::Write;

    /// Tạo 1 file .cab thật bằng chính API ghi của crate `cab` — xác nhận đường đọc của
    /// module này (viết ở trên) hoạt động đúng với dữ liệu do 1 thư viện độc lập tạo ra,
    /// không phải tự nối byte tay. `CompressionType::None` để test tập trung vào đúng cơ chế
    /// container CAB (tên file, kích thước, nội dung), không lẫn với việc kiểm chứng riêng
    /// bộ giải nén MSZIP/LZX của chính crate `cab` (đã là trách nhiệm của crate đó).
    fn make_sample_cab(path: &Path) {
        let mut builder = CabinetBuilder::new();
        let folder = builder.add_folder(CompressionType::None);
        folder.add_file("hello.txt");
        folder.add_file("sub\\nested.txt");

        let file = File::create(path).unwrap();
        let mut writer = builder.build(file).unwrap();

        let mut f = writer.next_file().unwrap().unwrap();
        assert_eq!(f.file_name(), "hello.txt");
        f.write_all(b"xin chao tu cab").unwrap();

        let mut f = writer.next_file().unwrap().unwrap();
        assert_eq!(f.file_name(), "sub\\nested.txt");
        f.write_all(b"file trong thu muc con").unwrap();

        writer.finish().unwrap();
    }

    #[test]
    fn list_extract_and_test_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("sample.cab");
        make_sample_cab(&archive);

        let entries = list_entries(&archive).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.name == "hello.txt" && e.size == 15));
        assert!(entries.iter().any(|e| e.name == "sub/nested.txt"));

        assert!(test_integrity(&archive).unwrap());

        let dest = tmp.path().join("out");
        extract(&archive, &dest).unwrap();
        assert_eq!(std::fs::read_to_string(dest.join("hello.txt")).unwrap(), "xin chao tu cab");
        assert_eq!(
            std::fs::read_to_string(dest.join("sub/nested.txt")).unwrap(),
            "file trong thu muc con"
        );
    }

    #[test]
    fn works_through_public_core_api() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("sample.cab");
        make_sample_cab(&archive);

        let entries = crate::list_entries(&archive, None).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(crate::test_integrity(&archive, None).unwrap());

        let dest = tmp.path().join("out");
        crate::extract(&archive, &dest, None).unwrap();
        assert!(dest.join("hello.txt").exists());
    }
}
