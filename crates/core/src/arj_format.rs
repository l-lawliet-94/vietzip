//! Định dạng `.arj` (ARJ, trình nén thời MS-DOS) — chỉ giải nén, mở rộng theo yêu cầu phủ
//! danh sách định dạng 7-Zip hỗ trợ (xem CLAUDE.md mục "Broader extract-only format coverage").
//!
//! Dùng crate `unarc-rs` (MIT/Apache-2.0, pure Rust) — chỉ dùng riêng module con `arj` của nó
//! (`unarc_rs::arj::arj_archive::ArjArchive`), KHÔNG dùng các module định dạng khác crate này
//! cũng hỗ trợ (7z/ZIP/RAR/LHA/...) — dự án đã có module riêng, đã kiểm chứng kỹ hơn cho từng
//! định dạng đó rồi (`zip_format.rs`, `sevenz_format.rs`, `rar_format.rs`, `lzh_format.rs`).
//!
//! **Rủi ro đã xác nhận, chấp nhận có chủ đích, không giấu**: `unarc-rs` có
//! `[features] default = []` — không có cách nào chỉ bật riêng ARJ, crate kéo theo BẮT BUỘC
//! toàn bộ dependency cho mọi định dạng nó hỗ trợ, gồm cả crate `unrar` 0.5.8 (KHÁC `unrar-ng`
//! dự án đang dùng cho `rar_format.rs`). Đã xác nhận trực tiếp: `unrar_sys-0.5.8`'s `build.rs`
//! dùng `cfg!(windows)` (dựa vào máy HOST, không phải TARGET biên dịch) — **đúng lỗi đã gặp và
//! phải vá ở `vendor/unrar-ng-sys`** để cross-compile Android được. Nghĩa là build Android sẽ
//! lỗi lại đúng kiểu lỗi đó cho tới khi `unrar_sys` được vá tương tự (chưa làm — chưa cần thiết
//! cho tới khi Android thực sự được build lại, xem "Status by platform"; người dùng đã được
//! báo rõ điều này và xác nhận muốn thêm ARJ ngay trên Windows/Linux/macOS, xử lý vá Android
//! sau). Cũng kéo theo 1 bản `sevenz-rust2` 0.21.4 khác song song với bản 0.7 dự án đang dùng
//! trực tiếp — không xung đột biên dịch (Cargo coi 2 major version là 2 crate riêng), chỉ tăng
//! kích thước binary.
//!
//! API tuần tự dựa trên cursor (`get_next_entry`/`read`/`skip`), giống `cpio_format.rs`/
//! `lzh_format.rs` — không random-access theo tên.
//!
//! **Có hỗ trợ mật khẩu thật** (khác `.cab`/`.cpio`/`.deb`/`.rpm`/`.lzh`/ext — những định dạng
//! đó dự án chưa wire mật khẩu vì bản thân crate đọc không hỗ trợ hoặc chưa cần): `ArjArchive`
//! có `set_password`/`read_with_password`, tự phân biệt "cần mật khẩu" (`is_garbled() &&
//! password.is_none()`) và "sai mật khẩu" qua các biến thể lỗi riêng (`ArchiveError::
//! PasswordRequired`/`EncryptionRequired`/`InvalidPassword`) — ánh xạ thẳng sang
//! `Error::PasswordRequired`/`Error::WrongPassword` của dự án, cùng khớp UX với zip/7z/rar.

use crate::{EntryInfo, Error, Result};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use unarc_rs::arj::arj_archive::ArjArchive;
use unarc_rs::arj::local_file_header::{FileType, LocalFileHeader};
use unarc_rs::error::ArchiveError;

fn map_err(err: ArchiveError) -> Error {
    match err {
        ArchiveError::PasswordRequired { .. } | ArchiveError::EncryptionRequired { .. } => {
            Error::PasswordRequired
        }
        ArchiveError::InvalidPassword { .. } => Error::WrongPassword,
        other => Error::Archive(other.to_string()),
    }
}

fn open(archive_path: &Path, password: Option<&str>) -> Result<ArjArchive<File>> {
    let file = File::open(archive_path).map_err(|e| Error::io(archive_path, e))?;
    let mut archive = ArjArchive::new(file).map_err(map_err)?;
    if let Some(pw) = password {
        archive.set_password(pw);
    }
    Ok(archive)
}

/// ARJ (thời MS-DOS) thường dùng `\` làm dấu phân cách thư mục trong tên entry — chuẩn hoá về
/// `/` trước khi lọc component, khớp cách `zip_format.rs::zip_entry_name` chuẩn hoá chiều
/// ngược lại. Đồng thời lọc bỏ `..`/tuyệt đối — `unarc-rs` không tự làm việc này (tên chỉ là
/// `String` thô), nên đây là lớp phòng vệ path-traversal của module này, cùng kiểu đã áp dụng
/// ở `rpm_format.rs`/`lzh_format.rs`.
fn relative_path(name: &str) -> PathBuf {
    Path::new(&name.replace('\\', "/"))
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .collect()
}

pub fn extract(archive_path: &Path, dest_dir: &Path, password: Option<&str>) -> Result<()> {
    let mut archive = open(archive_path, password)?;
    fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;

    while let Some(header) = archive.get_next_entry().map_err(map_err)? {
        let rel = relative_path(&header.name);
        if rel.as_os_str().is_empty() {
            archive.skip(&header).map_err(map_err)?;
            continue;
        }
        let out_path = dest_dir.join(&rel);
        if header.file_type == FileType::Directory {
            fs::create_dir_all(&out_path).map_err(|e| Error::io(&out_path, e))?;
            archive.skip(&header).map_err(map_err)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
            }
            let data = archive.read(&header).map_err(map_err)?;
            let mut out_file = File::create(&out_path).map_err(|e| Error::io(&out_path, e))?;
            out_file.write_all(&data).map_err(|e| Error::io(&out_path, e))?;
        }
    }
    Ok(())
}

fn to_entry_info(header: &LocalFileHeader) -> Option<EntryInfo> {
    let rel = relative_path(&header.name);
    if rel.as_os_str().is_empty() {
        return None;
    }
    Some(EntryInfo {
        name: rel.to_string_lossy().replace('\\', "/"),
        size: header.original_size as u64,
        is_dir: header.file_type == FileType::Directory,
    })
}

pub fn list_entries(archive_path: &Path, password: Option<&str>) -> Result<Vec<EntryInfo>> {
    let mut archive = open(archive_path, password)?;
    let mut entries = Vec::new();
    while let Some(header) = archive.get_next_entry().map_err(map_err)? {
        if let Some(entry) = to_entry_info(&header) {
            entries.push(entry);
        }
        archive.skip(&header).map_err(map_err)?;
    }
    Ok(entries)
}

pub fn test_integrity(archive_path: &Path, password: Option<&str>) -> Result<bool> {
    let mut archive = open(archive_path, password)?;
    while let Some(header) = archive.get_next_entry().map_err(map_err)? {
        if header.file_type == FileType::Directory {
            archive.skip(&header).map_err(map_err)?;
        } else {
            archive.read(&header).map_err(map_err)?;
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture ARJ THẬT, không phải tự dựng bằng tay: sao chép nguyên vẹn từ chính nội dung đã
    /// PUBLISH của crate `unarc-rs` (`tests/arj/*.arj`, không nằm trong `exclude` của
    /// `Cargo.toml` nên là nội dung crates.io thật đã tải về, cùng giấy phép MIT/Apache-2.0),
    /// dùng cho chính bộ test của crate đó (`tests/arj_decompression.rs`/`arj_failures.rs`) —
    /// không phải suy đoán nội dung. `stored.arj` (method 0, không nén) chứa đúng 1 entry tên
    /// "LICENSE" với nội dung khớp file LICENSE gốc của `unarc-rs`.
    #[test]
    fn list_extract_and_test_roundtrip_on_real_stored_fixture() {
        let tmp = tempfile::tempdir().unwrap();
        let archive_bytes = include_bytes!("../tests/data/unarc_rs_stored.arj");
        let archive = tmp.path().join("stored.arj");
        fs::write(&archive, archive_bytes).unwrap();

        assert!(crate::test_integrity(&archive, None).unwrap());

        let entries = crate::list_entries(&archive, None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "LICENSE");
        assert!(!entries[0].is_dir);

        let dest = tmp.path().join("out");
        crate::extract(&archive, &dest, None).unwrap();
        let extracted = fs::read(dest.join("LICENSE")).unwrap();
        assert_eq!(extracted.len(), entries[0].size as usize);
        assert!(!extracted.is_empty());
    }

    /// Entry có CRC sai (fixture thật của chính `unarc-rs`, dùng cho `tests/arj_failures.rs`
    /// của nó) -> `test_integrity`/`extract` phải phát hiện lỗi, không âm thầm trả về dữ liệu
    /// sai.
    #[test]
    fn detects_wrong_crc32_as_corruption() {
        let tmp = tempfile::tempdir().unwrap();
        let archive_bytes = include_bytes!("../tests/data/unarc_rs_wrongcrc32.arj");
        let archive = tmp.path().join("wrongcrc32.arj");
        fs::write(&archive, archive_bytes).unwrap();

        let err = test_integrity(&archive, None).unwrap_err();
        assert!(matches!(err, Error::Archive(_)));
    }

    /// Entry mã hoá thật (fixture thật của `unarc-rs`), không đưa mật khẩu -> phải báo
    /// `PasswordRequired` rõ ràng, không đọc lén/đọc ra dữ liệu rác.
    #[test]
    fn encrypted_entry_without_password_reports_password_required() {
        let tmp = tempfile::tempdir().unwrap();
        let archive_bytes = include_bytes!("../tests/data/unarc_rs_license_crypted.arj");
        let archive = tmp.path().join("license_crypted.arj");
        fs::write(&archive, archive_bytes).unwrap();

        let err = crate::extract(&archive, &tmp.path().join("out"), None).unwrap_err();
        assert!(matches!(err, Error::PasswordRequired), "expected PasswordRequired, got {err:?}");
    }
}
