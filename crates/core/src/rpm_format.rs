//! Gói `.rpm` (Red Hat/Fedora/openSUSE) — chỉ giải nén, mở rộng theo yêu cầu người dùng phủ
//! toàn bộ danh sách định dạng 7-Zip hỗ trợ (xem CLAUDE.md mục "7-Zip feature parity").
//!
//! Dùng crate `rpm` (Apache-2.0/MIT, đọc/ghi RPM thật, không phải tự viết parser nhị phân
//! mới). Cố tình dùng API **streaming** (`PackageReader`/`next_file`), không dùng API đơn
//! giản hơn `Package::open`/`Package::files()` — API đơn giản đó tải nguyên payload đã nén
//! VÀ đã giải nén vào RAM (`Vec<u8>`) cùng lúc, vi phạm nguyên tắc streaming I/O đã áp dụng
//! xuyên suốt dự án cho file lớn (DoD #4, xem `lib.rs::large_file_tests`). `list_entries`
//! dùng `PackageMetadata::open` + `get_file_entries()` — đọc thẳng từ header, không đụng tới
//! payload nén chút nào, còn rẻ hơn cả streaming.
//!
//! Loại `xz-compression` khỏi feature set của crate `rpm` (xem `Cargo.toml`) — kéo theo
//! `liblzma` (FFI ràng buộc C), trái nguyên tắc pure-Rust đã chọn cho `.xz` ở `single_format.rs`.
//! Đổi lại: `.rpm` nén payload bằng XZ (phổ biến ở openSUSE) sẽ báo lỗi rõ ràng từ chính crate
//! khi giải nén, không phải đọc sai âm thầm — gzip/zstd/bzip2 (các kiểu nén phổ biến còn lại)
//! vẫn hoạt động bình thường.
//!
//! Đường dẫn cài đặt trong RPM luôn là đường dẫn tuyệt đối kiểu Unix (`/usr/bin/hello`) — nếu
//! nối thẳng vào `dest_dir` bằng `Path::join`, phần tuyệt đối sẽ THAY THẾ hoàn toàn `dest_dir`
//! (đúng theo ngữ nghĩa `Path::join` của Rust) — đây là 1 dạng lỗ hổng path-traversal thật sự,
//! không phải lý thuyết. `relative_path()` (dưới) lọc bỏ mọi `RootDir`/`ParentDir`/`Prefix`,
//! chỉ giữ lại các đoạn `Normal`, chặn cả path-traversal kiểu `..` từ 1 file .rpm hỏng/độc hại.

use crate::{EntryInfo, Error, Result};
use rpm::{FileType, PackageMetadata, PackageReader};
use std::fs::File;
use std::io;
use std::path::{Component, Path, PathBuf};

fn relative_path(path: &Path) -> PathBuf {
    path.components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .collect()
}

fn to_rpm_error(archive_path: &Path, err: rpm::Error) -> Error {
    Error::io(archive_path, io::Error::other(err.to_string()))
}

pub fn extract(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;
    let mut reader = PackageReader::open(archive_path).map_err(|e| to_rpm_error(archive_path, e))?;

    while let Some(mut file) = reader.next_file().map_err(|e| to_rpm_error(archive_path, e))? {
        let rel = relative_path(&file.metadata.path());
        if rel.as_os_str().is_empty() {
            continue;
        }
        let dest_path = dest_dir.join(&rel);

        if file.metadata.mode().file_type() == FileType::Dir {
            std::fs::create_dir_all(&dest_path).map_err(|e| Error::io(&dest_path, e))?;
            continue;
        }
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let mut out = File::create(&dest_path).map_err(|e| Error::io(&dest_path, e))?;
        io::copy(&mut file, &mut out).map_err(|e| Error::io(&dest_path, e))?;
    }
    Ok(())
}

pub fn list_entries(archive_path: &Path) -> Result<Vec<EntryInfo>> {
    let metadata = PackageMetadata::open(archive_path).map_err(|e| to_rpm_error(archive_path, e))?;
    let entries = metadata.get_file_entries().map_err(|e| to_rpm_error(archive_path, e))?;

    Ok(entries
        .into_iter()
        .filter_map(|entry| {
            let rel = relative_path(&entry.path());
            if rel.as_os_str().is_empty() {
                return None;
            }
            Some(EntryInfo {
                name: rel.to_string_lossy().replace('\\', "/"),
                size: entry.size() as u64,
                is_dir: entry.mode().file_type() == FileType::Dir,
            })
        })
        .collect())
}

pub fn test_integrity(archive_path: &Path) -> Result<bool> {
    let mut reader = PackageReader::open(archive_path).map_err(|e| to_rpm_error(archive_path, e))?;
    while let Some(mut file) = reader.next_file().map_err(|e| to_rpm_error(archive_path, e))? {
        if file.metadata.mode().file_type() != FileType::Dir {
            io::copy(&mut file, &mut io::sink()).map_err(|e| Error::io(archive_path, e))?;
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tạo 1 file .rpm thật bằng chính API ghi (`PackageBuilder`) của crate `rpm` — không tự
    /// nối byte tay, khớp cách các test định dạng mở rộng khác trong dự án luôn dựng dữ liệu
    /// qua 1 thư viện độc lập rồi mới đọc lại bằng module của mình.
    fn make_sample_rpm(path: &Path) {
        let pkg = rpm::PackageBuilder::new("vietzip-test", "1.0.0", "MIT", "noarch", "goi test")
            .with_file_contents(
                b"noi dung file that su can cai dat".to_vec(),
                rpm::FileOptions::new("/usr/bin/hello"),
            )
            .unwrap()
            .with_dir_entry(rpm::FileOptions::dir("/usr/share/vietzip-test"))
            .unwrap()
            .build()
            .unwrap();
        pkg.write_file(path).unwrap();
    }

    #[test]
    fn list_extract_and_test_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("sample.rpm");
        make_sample_rpm(&archive);

        let entries = list_entries(&archive).unwrap();
        assert!(entries.iter().any(|e| {
            e.name == "usr/bin/hello" && !e.is_dir && e.size == b"noi dung file that su can cai dat".len() as u64
        }));
        assert!(entries.iter().any(|e| e.name == "usr/share/vietzip-test" && e.is_dir));

        assert!(test_integrity(&archive).unwrap());

        let dest = tmp.path().join("out");
        extract(&archive, &dest).unwrap();
        assert_eq!(
            std::fs::read_to_string(dest.join("usr/bin/hello")).unwrap(),
            "noi dung file that su can cai dat"
        );
        assert!(dest.join("usr/share/vietzip-test").is_dir());
    }

    #[test]
    fn works_through_public_core_api() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("sample.rpm");
        make_sample_rpm(&archive);

        assert!(crate::test_integrity(&archive, None).unwrap());
        let dest = tmp.path().join("out");
        crate::extract(&archive, &dest, None).unwrap();
        assert!(dest.join("usr/bin/hello").exists());
    }
}
