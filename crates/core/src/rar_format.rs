//! Module RAR — CHỈ giải nén, không bao giờ tạo/ghi file .rar. FR-14.
//! Xem LICENSES.md: crate `unrar-ng` nhúng UnRAR source (giấy phép riêng của RARLAB, y hệt
//! `unrar` gốc — đã đối chiếu license.txt byte-for-byte) — được phép dùng free để xử lý RAR,
//! cấm dùng để phát triển phần mềm nén tương thích RAR. Dùng `unrar-ng` thay vì `unrar` gốc vì
//! build.rs của `unrar` biên dịch không điều kiện các file C++ chỉ dành cho Windows
//! (isnt.cpp dùng DWORD/OSVERSIONINFO/wbemidl.h) khiến build lỗi khi cross-compile sang
//! Android — `unrar-ng` cfg-gate đúng các file này theo target platform.

use crate::{EntryInfo, Error, Result};
use std::path::Path;
use unrar_ng::error::Code;
use unrar_ng::Archive;

fn map_err(err: unrar_ng::error::UnrarError) -> Error {
    match err.code {
        Code::MissingPassword => Error::PasswordRequired,
        Code::BadPassword => Error::WrongPassword,
        _ => Error::Archive(err.to_string()),
    }
}

fn open_archive<'a>(archive_path: &'a Path, password: Option<&'a str>) -> Archive<'a> {
    match password {
        Some(pw) => Archive::with_password(archive_path, pw),
        None => Archive::new(archive_path),
    }
}

pub fn extract(archive_path: &Path, dest_dir: &Path, password: Option<&str>) -> Result<()> {
    std::fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;
    let mut current = open_archive(archive_path, password)
        .open_for_processing()
        .map_err(map_err)?;
    while let Some(with_header) = current.read_header().map_err(map_err)? {
        current = with_header.extract_with_base(dest_dir).map_err(map_err)?;
    }
    Ok(())
}

pub fn list_entries(archive_path: &Path, password: Option<&str>) -> Result<Vec<EntryInfo>> {
    let listing = open_archive(archive_path, password)
        .open_for_listing()
        .map_err(map_err)?;
    let mut entries = Vec::new();
    for header in listing {
        let header = header.map_err(map_err)?;
        entries.push(EntryInfo {
            name: header.filename.to_string_lossy().into_owned(),
            size: header.unpacked_size,
            is_dir: header.is_directory(),
        });
    }
    Ok(entries)
}

pub fn test_integrity(archive_path: &Path, password: Option<&str>) -> Result<bool> {
    let mut current = open_archive(archive_path, password)
        .open_for_processing()
        .map_err(map_err)?;
    while let Some(with_header) = current.read_header().map_err(map_err)? {
        current = with_header.test().map_err(map_err)?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{extract, list_entries, test_integrity};
    use std::path::PathBuf;
    use std::process::Command;

    /// Đường dẫn Rar.exe thật (WinRAR) đã cài sẵn trên máy — dùng để tạo file .rar
    /// mẫu THẬT cho test, đúng yêu cầu DoD #3 trong ke-hoach-mvp.md ("giải nén đúng
    /// file .rar được tạo thật bởi WinRAR"). Không tự tải/tạo unrar bằng code của
    /// dự án vì dự án này không bao giờ tạo file .rar (xem FR-02, FR-14).
    const RAR_EXE: &str = r"C:\Program Files\WinRAR\Rar.exe";

    /// Tạo 1 file .rar mẫu bằng WinRAR thật. Trả về `None` (bỏ qua test) nếu máy
    /// không có WinRAR cài sẵn — test này phụ thuộc môi trường, không phải core logic.
    fn make_sample_rar_with_winrar(tmp_dir: &Path) -> Option<PathBuf> {
        if !Path::new(RAR_EXE).exists() {
            return None;
        }
        let src_dir = tmp_dir.join("src");
        std::fs::create_dir_all(src_dir.join("thư mục con")).unwrap();
        std::fs::write(src_dir.join("hello.txt"), b"xin chao viet nam").unwrap();
        std::fs::write(
            src_dir.join("thư mục con/tệp có dấu.txt"),
            "nội dung tiếng Việt".as_bytes(),
        )
        .unwrap();

        let archive = tmp_dir.join("sample.rar");
        let status = Command::new(RAR_EXE)
            .args(["a", "-r", "-ep1"])
            .arg(&archive)
            .arg(&src_dir)
            .status()
            .expect("chạy được Rar.exe");
        assert!(status.success(), "Rar.exe tạo file mẫu thất bại");
        Some(archive)
    }

    #[test]
    fn roundtrip_real_winrar_archive() {
        let tmp = tempfile::tempdir().unwrap();
        let Some(archive) = make_sample_rar_with_winrar(tmp.path()) else {
            eprintln!("Bỏ qua test: không tìm thấy WinRAR tại {RAR_EXE}");
            return;
        };

        assert!(test_integrity(&archive, None).unwrap());

        let entries = list_entries(&archive, None).unwrap();
        assert!(entries.iter().any(|e| e.name.ends_with("hello.txt")));
        assert!(entries
            .iter()
            .any(|e| e.name.contains("tệp có dấu.txt")));

        let dest_dir = tmp.path().join("out");
        extract(&archive, &dest_dir, None).unwrap();
        // -ep1 giữ nguyên tên thư mục gốc ("src") trong archive, giống cách zip/7z test lưu.
        let extracted = std::fs::read_to_string(dest_dir.join("src/hello.txt")).unwrap();
        assert_eq!(extracted, "xin chao viet nam");
        let extracted_vn =
            std::fs::read_to_string(dest_dir.join("src/thư mục con/tệp có dấu.txt")).unwrap();
        assert_eq!(extracted_vn, "nội dung tiếng Việt");
    }

    #[test]
    fn password_protected_real_winrar_archive() {
        if !Path::new(RAR_EXE).exists() {
            eprintln!("Bỏ qua test: không tìm thấy WinRAR tại {RAR_EXE}");
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("hello.txt"), b"xin chao viet nam").unwrap();

        let archive = tmp.path().join("secret.rar");
        let status = Command::new(RAR_EXE)
            .args(["a", "-r", "-ep1", "-hpmat-khau-vi-du"])
            .arg(&archive)
            .arg(&src_dir)
            .status()
            .expect("chạy được Rar.exe");
        assert!(status.success(), "Rar.exe tạo file mẫu thất bại");

        let err = extract(&archive, &tmp.path().join("wrong"), None).unwrap_err();
        assert!(matches!(err, Error::PasswordRequired));

        let err = extract(&archive, &tmp.path().join("wrong2"), Some("sai-roi")).unwrap_err();
        assert!(matches!(err, Error::WrongPassword));

        let dest_dir = tmp.path().join("ok");
        extract(&archive, &dest_dir, Some("mat-khau-vi-du")).unwrap();
        let extracted = std::fs::read_to_string(dest_dir.join("src/hello.txt")).unwrap();
        assert_eq!(extracted, "xin chao viet nam");
    }
}
