//! Module 7Z (LZMA2) — nén và giải nén. FR-01, FR-02, FR-05, FR-10, FR-11, FR-12, FR-16.
//! Mật khẩu + AES-256 dùng feature `aes256` của sevenz-rust2: mã hóa cả nội dung lẫn
//! header (ẩn luôn danh sách tên file — tương đương FR-06 "miễn phí" khi đã bật mật khẩu
//! cho 7z, xem ke-hoach-mvp.md mục 3 — không phải tính năng riêng, chỉ là hệ quả tự nhiên
//! của cách sevenz-rust2 mã hóa header).

use crate::{CompressOptions, CompressionLevel, EntryInfo, Error, Result};
use sevenz_rust2::encoder_options::{AesEncoderOptions, Lzma2Options};
use sevenz_rust2::{ArchiveEntry, ArchiveReader, ArchiveWriter, Password};
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

fn map_err(err: sevenz_rust2::Error) -> Error {
    match err {
        sevenz_rust2::Error::PasswordRequired => Error::PasswordRequired,
        sevenz_rust2::Error::MaybeBadPassword(_) => Error::WrongPassword,
        other => Error::Archive(other.to_string()),
    }
}

/// FR-03: LZMA2 nhận preset 0-9. Cùng thang 1/6/9 như ZIP (zip_format::deflate_level) để
/// nhất quán giữa các định dạng.
fn lzma2_preset(level: CompressionLevel) -> u32 {
    match level {
        CompressionLevel::Fast => 1,
        CompressionLevel::Normal => 6,
        CompressionLevel::Ultra => 9,
    }
}

/// sevenz-rust2 >= 0.17 nén LZMA2 đa luồng thật (`Lzma2Options::from_level_mt`, xem
/// CHANGELOG + `lzma-rust2`'s `work_pool.rs`/`work_queue.rs` — thread pool thật, không phải
/// giả). `chunk_size` sẽ tự bị kẹp lên tối thiểu bằng dictionary size của preset đang dùng
/// (đọc trực tiếp từ source `encoder_options.rs`), nên 1 MiB ở đây chỉ là sàn — preset Ultra
/// (dict lớn) vẫn tạo chunk lớn hơn tương ứng, không hy sinh tỷ lệ nén một cách vô ích.
/// Áp dụng cho MỌI kích thước nguồn (nhiều file lẫn 1 file khổng lồ) vì việc chia luồng nằm
/// ngay trong codec, không phải ở tầng "nhiều file -> nhiều thread" như zip_format.rs.
const LZMA2_MT_CHUNK_SIZE: u64 = 1 << 20;

fn lzma2_options(level: CompressionLevel) -> Lzma2Options {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1);
    Lzma2Options::from_level_mt(lzma2_preset(level), threads, LZMA2_MT_CHUNK_SIZE)
}

pub fn compress(sources: &[PathBuf], dest: &Path, options: &CompressOptions) -> Result<()> {
    let mut writer = ArchiveWriter::create(dest).map_err(map_err)?;

    let lzma2 = lzma2_options(options.level);
    if let Some(password) = &options.password {
        // Thứ tự trong Vec quan trọng: phần tử cuối áp dụng TRƯỚC (lên dữ liệu gốc), phần
        // tử đầu áp dụng SAU (gần đĩa nhất) — nên nén trước (LZMA2, cuối) rồi mới mã hóa
        // (AES, đầu). Đúng theo ví dụ chính thức examples/advance.rs của sevenz-rust2.
        writer.set_encrypt_header(true);
        writer.set_content_methods(vec![
            AesEncoderOptions::new(Password::from(password.as_str())).into(),
            lzma2.into(),
        ]);
    } else {
        writer.set_content_methods(vec![lzma2.into()]);
    }

    for source in sources {
        let base_name = source
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| source.clone());
        add_path(&mut writer, source, &base_name)?;
    }

    writer.finish().map_err(|e| Error::io(dest, e))?;
    Ok(())
}

fn add_path(writer: &mut ArchiveWriter<File>, fs_path: &Path, entry_path: &Path) -> Result<()> {
    let name = entry_path.to_string_lossy().replace('\\', "/");
    if fs_path.is_dir() {
        writer
            .push_archive_entry::<File>(ArchiveEntry::new_directory(&name), None)
            .map_err(map_err)?;
        let mut children: Vec<_> = std::fs::read_dir(fs_path)
            .map_err(|e| Error::io(fs_path, e))?
            .filter_map(|e| e.ok())
            .collect();
        children.sort_by_key(|e| e.file_name());
        for child in children {
            let child_fs_path = child.path();
            let child_entry_path = entry_path.join(child.file_name());
            add_path(writer, &child_fs_path, &child_entry_path)?;
        }
    } else {
        let file = File::open(fs_path).map_err(|e| Error::io(fs_path, e))?;
        writer
            .push_archive_entry(ArchiveEntry::new_file(&name), Some(file))
            .map_err(map_err)?;
    }
    Ok(())
}

fn open_reader(archive_path: &Path, password: Option<&str>) -> Result<ArchiveReader<File>> {
    let password = match password {
        Some(pw) => Password::from(pw),
        None => Password::empty(),
    };
    ArchiveReader::open(archive_path, password).map_err(map_err)
}

pub fn extract(archive_path: &Path, dest_dir: &Path, password: Option<&str>) -> Result<()> {
    std::fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;
    let mut reader = open_reader(archive_path, password)?;
    reader
        .for_each_entries(|entry, data| {
            let out_path = dest_dir.join(&entry.name);
            if entry.is_directory {
                std::fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut out_file = File::create(&out_path)?;
                io::copy(data, &mut out_file)?;
            }
            Ok(true)
        })
        .map_err(map_err)
}

pub fn list_entries(archive_path: &Path, password: Option<&str>) -> Result<Vec<EntryInfo>> {
    let reader = open_reader(archive_path, password)?;
    Ok(reader
        .archive()
        .files
        .iter()
        .map(|entry| EntryInfo {
            name: entry.name.clone(),
            size: entry.size,
            is_dir: entry.is_directory,
        })
        .collect())
}

pub fn test_integrity(archive_path: &Path, password: Option<&str>) -> Result<bool> {
    let mut reader = open_reader(archive_path, password)?;
    reader
        .for_each_entries(|_entry, data| {
            io::copy(data, &mut io::sink())?;
            Ok(true)
        })
        .map_err(map_err)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compress, extract, list_entries, test_integrity};

    fn make_sample_dir(root: &Path) {
        std::fs::create_dir_all(root.join("thư mục con")).unwrap();
        std::fs::write(root.join("hello.txt"), b"xin chao viet nam").unwrap();
        std::fs::write(
            root.join("thư mục con/tệp có dấu.txt"),
            "nội dung tiếng Việt".as_bytes(),
        )
        .unwrap();
    }

    #[test]
    fn roundtrip_7z() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        make_sample_dir(&src_dir);

        let archive = tmp.path().join("out.7z");
        let options = CompressOptions::default();
        compress(&[src_dir], &archive, &options).unwrap();

        assert!(test_integrity(&archive, None).unwrap());

        let entries = list_entries(&archive, None).unwrap();
        assert!(entries.iter().any(|e| e.name.contains("hello.txt")));
        assert!(entries.iter().any(|e| e.name.contains("tệp có dấu.txt")));

        let dest_dir = tmp.path().join("out");
        extract(&archive, &dest_dir, None).unwrap();
        let extracted = std::fs::read_to_string(dest_dir.join("src/hello.txt")).unwrap();
        assert_eq!(extracted, "xin chao viet nam");
        let extracted_vn =
            std::fs::read_to_string(dest_dir.join("src/thư mục con/tệp có dấu.txt")).unwrap();
        assert_eq!(extracted_vn, "nội dung tiếng Việt");
    }

    #[test]
    fn roundtrip_7z_with_aes256_password() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        make_sample_dir(&src_dir);

        let archive = tmp.path().join("secret.7z");
        let options = CompressOptions {
            password: Some("mat-khau-vi-du".to_string()),
            ..Default::default()
        };
        compress(&[src_dir], &archive, &options).unwrap();

        // Không có mật khẩu -> phải báo cần mật khẩu, không được đọc lén được (kể cả
        // liệt kê tên file — header cũng bị mã hóa nhờ set_encrypt_header(true)).
        let err = list_entries(&archive, None).unwrap_err();
        assert!(matches!(err, Error::PasswordRequired));

        // Sai mật khẩu -> báo sai mật khẩu.
        let err = extract(&archive, &tmp.path().join("wrong"), Some("sai-roi")).unwrap_err();
        assert!(matches!(err, Error::WrongPassword));

        // Đúng mật khẩu -> giải nén thành công, nội dung đúng.
        let dest_dir = tmp.path().join("ok");
        extract(&archive, &dest_dir, Some("mat-khau-vi-du")).unwrap();
        let extracted = std::fs::read_to_string(dest_dir.join("src/hello.txt")).unwrap();
        assert_eq!(extracted, "xin chao viet nam");
    }
}
