//! Định dạng `.lzh`/`.lha` (LHA/LZH) — chỉ giải nén, mở rộng theo yêu cầu phủ danh sách định
//! dạng 7-Zip hỗ trợ (xem CLAUDE.md mục "Broader extract-only format coverage").
//!
//! Dùng crate `delharc` (MIT/Apache-2.0, pure Rust, không FFI — quan trọng vì Android vẫn còn
//! trong roadmap, xem `unrar-ng-sys`). API tuần tự (`LhaDecodeReader`, tương tự `cpio`/`ar`):
//! đọc từng entry 1 lần theo thứ tự trong file, không random-access theo tên — `extract`/
//! `list_entries`/`test_integrity` mỗi hàm tự lái vòng lặp riêng, cùng cách `cpio_format.rs`
//! đã làm cho lý do tương tự.
//!
//! **Giới hạn định dạng, không phải bug**: LHA/LZH (thiết kế từ thập niên 1990) không có cờ
//! UTF-8 cho tên file như ZIP — `delharc` percent-encode mọi byte ngoài ASCII in được thành
//! `%XX` khi parse tên (xác nhận qua đọc trực tiếp `delharc`'s test `path_parser_works`, vd
//! byte `0xff` → `%ff`). Nghĩa là tên file tiếng Việt có dấu (UTF-8 nhiều byte) sẽ không round-trip
//! nguyên vẹn qua định dạng này — đây là giới hạn cố hữu của bản thân định dạng LHA, không
//! phải lỗi implementation ở đây.

use crate::{EntryInfo, Error, Result};
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};

fn map_err<E: std::fmt::Display>(err: E) -> Error {
    Error::Archive(err.to_string())
}

/// Loại bỏ mọi component tuyệt đối/`..`/prefix ổ đĩa trước khi nối vào `dest_dir` — phòng vệ
/// thêm dù `delharc::LhaHeader::parse_pathname` bản thân nó đã tự bỏ qua `.`/`..` (xác nhận
/// qua đọc test `path_parser_works` của chính crate: `..`, `/../.\`, v.v. đều parse ra rỗng),
/// cùng kiểu "phòng vệ 2 lớp" đã áp dụng ở `rpm_format.rs::relative_path` cho RPM.
fn relative_path(path: &Path) -> PathBuf {
    path.components().filter(|c| matches!(c, Component::Normal(_))).collect()
}

pub fn extract(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;
    let mut reader = delharc::parse_file(archive_path).map_err(|e| Error::io(archive_path, e))?;

    loop {
        let header = reader.header().clone();
        let rel = relative_path(&header.parse_pathname());
        if !rel.as_os_str().is_empty() {
            let out_path = dest_dir.join(&rel);
            if header.is_directory() {
                fs::create_dir_all(&out_path).map_err(|e| Error::io(&out_path, e))?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
                }
                if !reader.is_decoder_supported() {
                    return Err(Error::Archive(format!(
                        "phương thức nén của '{}' trong file LZH/LHA này chưa được hỗ trợ",
                        rel.display()
                    )));
                }
                let mut out_file = File::create(&out_path).map_err(|e| Error::io(&out_path, e))?;
                io::copy(&mut reader, &mut out_file).map_err(map_err)?;
                reader.crc_check().map_err(map_err)?;
            }
        }

        if !reader.seek_next_file().map_err(map_err)? {
            break;
        }
    }
    Ok(())
}

pub fn list_entries(archive_path: &Path) -> Result<Vec<EntryInfo>> {
    let mut reader = delharc::parse_file(archive_path).map_err(|e| Error::io(archive_path, e))?;
    let mut entries = Vec::new();

    loop {
        let header = reader.header();
        let rel = relative_path(&header.parse_pathname());
        if !rel.as_os_str().is_empty() {
            entries.push(EntryInfo {
                name: rel.to_string_lossy().replace('\\', "/"),
                size: header.original_size,
                is_dir: header.is_directory(),
            });
        }
        if !reader.seek_next_file().map_err(map_err)? {
            break;
        }
    }
    Ok(entries)
}

pub fn test_integrity(archive_path: &Path) -> Result<bool> {
    let mut reader = delharc::parse_file(archive_path).map_err(|e| Error::io(archive_path, e))?;
    loop {
        if !reader.header().is_directory() {
            if !reader.is_decoder_supported() {
                return Err(Error::Archive(
                    "có entry dùng phương thức nén chưa được hỗ trợ".to_string(),
                ));
            }
            io::copy(&mut reader, &mut io::sink()).map_err(map_err)?;
            reader.crc_check().map_err(map_err)?;
        }
        if !reader.seek_next_file().map_err(map_err)? {
            break;
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use delharc::crc::Crc16;
    use std::io::Write;

    /// Dựng 1 file `.lzh` LEVEL 0, phương thức `-lh0-` (không nén) — cách đơn giản và an toàn
    /// nhất để có 1 fixture THẬT mà không cần hand-roll cả 1 bộ mã hoá LZ/Huffman.
    ///
    /// `delharc` không có API ghi (chỉ đọc — đúng bản chất "chỉ giải nén" của định dạng này
    /// trong dự án), và máy này không có công cụ LHA/LZH thật nào cài sẵn để shell ra như
    /// cách `rar_format.rs` dùng WinRAR thật. Vì vậy hàm này tự dựng đúng layout byte của
    /// header level 0, nhưng KHÔNG đoán mò: mọi field/độ dài được suy trực tiếp từ việc đọc
    /// `delharc::header::parser::LhaHeader::read` (constant offsets: base header 19 byte —
    /// 5+4+4+4+1+1, checksum là tổng wrapping mod-256 của toàn bộ phần thân header sau 2 byte
    /// đầu — xem `wrapping_csum` private của chính crate, thuật toán tầm thường nên tái hiện
    /// lại an toàn), và CRC-16 nội dung dùng thẳng `delharc::crc::Crc16` (public) — không tự
    /// implement thuật toán CRC. Cách này chỉ chấp nhận được vì đây là fixture TEST, dùng
    /// header level 0 đơn giản nhất (không nén, không extra header) — không phải code ghi
    /// dùng trong production, và mọi byte đều bắt nguồn trực tiếp từ chính parser thật sẽ đọc
    /// lại nó, không phải một cách đọc spec độc lập có thể sai lệch.
    fn build_minimal_lh0_lzh(filename: &[u8], content: &[u8]) -> Vec<u8> {
        assert!(filename.len() <= 255);
        let mut crc = Crc16::default();
        crc.digest(content);
        let file_crc = crc.sum16();

        let mut body = Vec::new(); // mọi thứ TRỪ header_len + csum (2 byte đầu)
        body.extend_from_slice(b"-lh0-"); // compression[5]
        body.extend_from_slice(&(content.len() as u32).to_le_bytes()); // compressed_size
        body.extend_from_slice(&(content.len() as u32).to_le_bytes()); // original_size
        body.extend_from_slice(&0u32.to_le_bytes()); // last_modified (khong dung trong test)
        body.push(0x20); // msdos_attrs (ARCHIVE)
        body.push(0); // lha_level = 0
        body.push(filename.len() as u8); // filename_len
        body.extend_from_slice(filename);
        body.extend_from_slice(&file_crc.to_le_bytes());

        let header_len = body.len() as u8; // level 0: khong extended area, khong extra header
        let csum = body.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));

        let mut out = Vec::new();
        out.push(header_len);
        out.push(csum);
        out.extend_from_slice(&body);
        out.extend_from_slice(content);
        out
    }

    fn make_sample_lzh(path: &Path) {
        let data = build_minimal_lh0_lzh(b"test.txt", b"noi dung khong nen (-lh0-)");
        let mut f = File::create(path).unwrap();
        f.write_all(&data).unwrap();
    }

    #[test]
    fn works_through_public_core_api() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("sample.lzh");
        make_sample_lzh(&archive);

        assert!(crate::test_integrity(&archive, None).unwrap());
        let entries = crate::list_entries(&archive, None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "test.txt");
        assert!(!entries[0].is_dir);

        let dest = tmp.path().join("out");
        crate::extract(&archive, &dest, None).unwrap();
        let content = fs::read_to_string(dest.join("test.txt")).unwrap();
        assert_eq!(content, "noi dung khong nen (-lh0-)");
    }

    #[test]
    fn list_extract_and_test_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("sample.lzh");
        make_sample_lzh(&archive);

        assert!(test_integrity(&archive).unwrap());

        let entries = list_entries(&archive).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "test.txt");
        assert_eq!(entries[0].size, "noi dung khong nen (-lh0-)".len() as u64);

        let dest = tmp.path().join("out");
        extract(&archive, &dest).unwrap();
        let content = fs::read_to_string(dest.join("test.txt")).unwrap();
        assert_eq!(content, "noi dung khong nen (-lh0-)");
    }

    /// Nội dung bị hỏng (CRC không khớp) phải bị phát hiện, không âm thầm giải nén ra dữ liệu sai.
    #[test]
    fn detects_crc_mismatch_as_corruption() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("corrupt.lzh");
        let mut data = build_minimal_lh0_lzh(b"bad.txt", b"noi dung goc");
        // Phá nội dung SAU khi CRC đã tính trên nội dung gốc — mô phỏng dữ liệu hỏng giữa chừng,
        // không đổi kích thước (giữ header hợp lệ) nên chỉ có bước kiểm CRC bắt được lỗi này.
        let content_start = data.len() - "noi dung goc".len();
        data[content_start] ^= 0xFF;
        fs::write(&archive, &data).unwrap();

        let err = test_integrity(&archive).unwrap_err();
        assert!(matches!(err, Error::Archive(_)));
    }
}
