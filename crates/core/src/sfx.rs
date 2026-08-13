//! Self-extracting archive (SFX) — FR-07.
//!
//! Không nhúng lại SFX module gốc của 7-Zip (bản quyền riêng, không thể tái sử dụng tự do
//! trong 1 dự án khác) — tự viết stub launcher tối giản (`crates/sfx-stub`, một binary
//! crate riêng phụ thuộc `vietzip_core`) và ghép nó với dữ liệu archive theo kỹ thuật SFX
//! kinh điển: nối thêm dữ liệu vào cuối 1 file PE hợp lệ. Windows PE loader chỉ đọc đến hết
//! section cuối cùng theo bảng section header, không quan tâm file có dài hơn — dữ liệu phụ
//! trội ở cuối là an toàn và là cách mọi SFX module (kể cả của 7-Zip/WinRAR) đều dùng.
//!
//! Định dạng trailer (27 byte cố định ở cuối file `.exe` đã tạo, cộng thêm phần lệnh chạy
//! sau giải nén nếu có — tương đương rút gọn của directive `RunProgram` trong SFX config.txt
//! của 7-Zip, xem CLAUDE.md mục "7-Zip feature parity" > Done > "SFX configurable install
//! behavior"):
//! `[stub][archive][run_cmd UTF-8, run_cmd_len byte][MAGIC 16][archive_len 8 LE][run_cmd_len 2 LE][format_tag 1]`
//!
//! Định dạng này do chính dự án tự định nghĩa (không phải chuẩn của bên thứ ba), nên việc mở
//! rộng thêm trường `run_cmd` không có rủi ro tương thích ngược với file nào khác ngoài
//! những file SFX mà chính công cụ này đã tạo ra trước đó — chưa có bản phát hành công khai
//! nào dùng định dạng cũ (27 byte thay vì 29 byte cũ), nên không cần giữ tương thích ngược.

use crate::{Error, Format, Result};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 16] = b"VIETZIPSFXv0001\0";
const FIXED_TRAILER_LEN: u64 = 16 + 8 + 2 + 1;
/// Giới hạn độ dài lệnh chạy sau giải nén — đủ cho bất kỳ đường dẫn/tham số thực tế nào,
/// tránh 1 giá trị `run_cmd_len` bất thường (file hỏng/bị sửa) khiến việc đọc trailer cố
/// đọc ngược quá xa vào phần dữ liệu archive.
const MAX_RUN_CMD_LEN: usize = 4096;

fn format_tag(format: Format) -> Result<u8> {
    match format {
        Format::Zip => Ok(1),
        Format::SevenZ => Ok(2),
        other => Err(Error::UnsupportedOperation(other)),
    }
}

fn tag_extension(tag: u8) -> Result<&'static str> {
    match tag {
        1 => Ok("zip"),
        2 => Ok("7z"),
        _ => Err(Error::Archive(format!("SFX: định dạng archive không hợp lệ (tag={tag})"))),
    }
}

/// Ghép `stub` (binary launcher đã biên dịch sẵn, xem `crates/sfx-stub`) với `archive`
/// (đã nén sẵn — `.zip` hoặc `.7z`) thành 1 file `.exe` tự giải nén tại `output`.
/// `run_after_extract`, nếu có, là đường dẫn (tương đối so với thư mục vừa giải nén) tới 1
/// chương trình mà stub sẽ tự chạy ngay sau khi giải nén xong — tương đương mức tối giản của
/// directive `RunProgram` trong SFX config.txt của 7-Zip, dùng cho kịch bản "file .exe tự
/// giải nén rồi tự cài đặt luôn" thay vì chỉ giải nén thụ động.
pub fn write_sfx(stub: &Path, archive: &Path, output: &Path, run_after_extract: Option<&str>) -> Result<()> {
    let format = Format::from_path(archive).ok_or_else(|| Error::UnknownFormat(archive.to_path_buf()))?;
    let tag = format_tag(format)?;

    let run_cmd_bytes = run_after_extract.unwrap_or("").as_bytes();
    if run_cmd_bytes.len() > MAX_RUN_CMD_LEN {
        return Err(Error::Archive(format!(
            "Lệnh chạy sau giải nén quá dài (tối đa {MAX_RUN_CMD_LEN} byte)"
        )));
    }
    let run_cmd_len: u16 = run_cmd_bytes.len() as u16;

    let mut stub_file = File::open(stub).map_err(|e| Error::io(stub, e))?;
    let mut archive_file = File::open(archive).map_err(|e| Error::io(archive, e))?;
    let archive_len = archive_file
        .metadata()
        .map_err(|e| Error::io(archive, e))?
        .len();

    let mut out = File::create(output).map_err(|e| Error::io(output, e))?;
    io::copy(&mut stub_file, &mut out).map_err(|e| Error::io(stub, e))?;
    io::copy(&mut archive_file, &mut out).map_err(|e| Error::io(archive, e))?;
    out.write_all(run_cmd_bytes).map_err(|e| Error::io(output, e))?;
    out.write_all(MAGIC).map_err(|e| Error::io(output, e))?;
    out.write_all(&archive_len.to_le_bytes())
        .map_err(|e| Error::io(output, e))?;
    out.write_all(&run_cmd_len.to_le_bytes())
        .map_err(|e| Error::io(output, e))?;
    out.write_all(&[tag]).map_err(|e| Error::io(output, e))?;

    Ok(())
}

/// Kết quả đọc trailer của 1 file SFX: đường dẫn thật đã trích xuất archive ra, và lệnh
/// (nếu có) cần chạy sau khi giải nén xong.
#[derive(Debug)]
pub struct SfxInfo {
    pub archive_path: PathBuf,
    pub run_after_extract: Option<String>,
}

/// Đọc trailer đã ghép ở cuối chính file thực thi đang chạy (`exe_path`, thường là
/// `std::env::current_exe()`) và trích xuất phần dữ liệu archive ra `dest_archive` (dùng
/// đúng đuôi mở rộng tương ứng để `Format::from_path` nhận diện được khi gọi `extract`).
pub fn extract_embedded_archive(exe_path: &Path, dest_archive: &Path) -> Result<SfxInfo> {
    let mut exe = File::open(exe_path).map_err(|e| Error::io(exe_path, e))?;
    let file_len = exe.metadata().map_err(|e| Error::io(exe_path, e))?.len();

    if file_len < FIXED_TRAILER_LEN {
        return Err(Error::Archive("Không phải file SFX hợp lệ (file quá ngắn)".to_string()));
    }
    exe.seek(SeekFrom::End(-(FIXED_TRAILER_LEN as i64)))
        .map_err(|e| Error::io(exe_path, e))?;
    let mut trailer = [0u8; FIXED_TRAILER_LEN as usize];
    exe.read_exact(&mut trailer).map_err(|e| Error::io(exe_path, e))?;

    if &trailer[0..16] != MAGIC {
        return Err(Error::Archive(
            "Không phải file SFX hợp lệ (thiếu magic trailer)".to_string(),
        ));
    }
    let archive_len = u64::from_le_bytes(trailer[16..24].try_into().unwrap());
    let run_cmd_len = u16::from_le_bytes(trailer[24..26].try_into().unwrap()) as u64;
    let tag = trailer[26];
    let ext = tag_extension(tag)?;

    if FIXED_TRAILER_LEN + run_cmd_len + archive_len > file_len {
        return Err(Error::Archive(
            "File SFX bị hỏng: kích thước trong trailer lớn hơn file".to_string(),
        ));
    }

    let run_cmd_start = file_len - FIXED_TRAILER_LEN - run_cmd_len;
    let archive_start = run_cmd_start - archive_len;

    let run_after_extract = if run_cmd_len > 0 {
        exe.seek(SeekFrom::Start(run_cmd_start))
            .map_err(|e| Error::io(exe_path, e))?;
        let mut buf = vec![0u8; run_cmd_len as usize];
        exe.read_exact(&mut buf).map_err(|e| Error::io(exe_path, e))?;
        Some(
            String::from_utf8(buf)
                .map_err(|_| Error::Archive("Lệnh chạy sau giải nén không phải UTF-8 hợp lệ".to_string()))?,
        )
    } else {
        None
    };

    let dest_archive = dest_archive.with_extension(ext);
    exe.seek(SeekFrom::Start(archive_start))
        .map_err(|e| Error::io(exe_path, e))?;
    let mut limited = exe.take(archive_len);
    let mut out = File::create(&dest_archive).map_err(|e| Error::io(&dest_archive, e))?;
    io::copy(&mut limited, &mut out).map_err(|e| Error::io(&dest_archive, e))?;

    Ok(SfxInfo { archive_path: dest_archive, run_after_extract })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stub_and_archive(tmp: &Path) -> (PathBuf, PathBuf) {
        let stub = tmp.join("stub.bin");
        std::fs::write(&stub, b"gia lap stub launcher exe, khong chay duoc that").unwrap();

        let src_dir = tmp.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("hello.txt"), b"xin chao viet nam").unwrap();
        let archive = tmp.join("out.zip");
        crate::compress(&[src_dir], &archive, &crate::CompressOptions::default()).unwrap();
        (stub, archive)
    }

    #[test]
    fn write_then_extract_embedded_roundtrip_no_run_command() {
        let tmp = tempfile::tempdir().unwrap();
        let (stub, archive) = make_stub_and_archive(tmp.path());

        let sfx_exe = tmp.path().join("installer.exe");
        write_sfx(&stub, &archive, &sfx_exe, None).unwrap();

        let sfx_len = std::fs::metadata(&sfx_exe).unwrap().len();
        let stub_len = std::fs::metadata(&stub).unwrap().len();
        let archive_len = std::fs::metadata(&archive).unwrap().len();
        assert_eq!(sfx_len, stub_len + archive_len + FIXED_TRAILER_LEN);

        let extracted_archive = tmp.path().join("extracted");
        let info = extract_embedded_archive(&sfx_exe, &extracted_archive).unwrap();
        assert!(info.archive_path.to_string_lossy().ends_with(".zip"));
        assert_eq!(info.run_after_extract, None);

        let dest_dir = tmp.path().join("out_dir");
        crate::extract(&info.archive_path, &dest_dir, None).unwrap();
        let content = std::fs::read_to_string(dest_dir.join("src/hello.txt")).unwrap();
        assert_eq!(content, "xin chao viet nam");
    }

    /// FR-07 mở rộng: SFX có thể tự chạy 1 chương trình ngay sau khi giải nén (tương đương
    /// rút gọn của `RunProgram` trong config.txt của 7-Zip).
    #[test]
    fn write_then_extract_embedded_roundtrip_with_run_command() {
        let tmp = tempfile::tempdir().unwrap();
        let (stub, archive) = make_stub_and_archive(tmp.path());

        let sfx_exe = tmp.path().join("installer.exe");
        write_sfx(&stub, &archive, &sfx_exe, Some("src\\hello.txt")).unwrap();

        let extracted_archive = tmp.path().join("extracted");
        let info = extract_embedded_archive(&sfx_exe, &extracted_archive).unwrap();
        assert_eq!(info.run_after_extract.as_deref(), Some("src\\hello.txt"));

        let dest_dir = tmp.path().join("out_dir");
        crate::extract(&info.archive_path, &dest_dir, None).unwrap();
        assert!(dest_dir.join("src/hello.txt").exists());
    }

    #[test]
    fn extract_embedded_rejects_non_sfx_file() {
        let tmp = tempfile::tempdir().unwrap();
        let not_sfx = tmp.path().join("plain.exe");
        std::fs::write(&not_sfx, b"khong phai sfx").unwrap();
        let err = extract_embedded_archive(&not_sfx, &tmp.path().join("out")).unwrap_err();
        assert!(matches!(err, Error::Archive(_)));
    }
}
