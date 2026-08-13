//! Nén/giải nén 1 file đơn lẻ dạng `.gz`/`.bz2`/`.zst` — khác với `.tar.gz`/`.tar.bz2`/
//! `.tar.zst` (nhiều file gói trong 1 tar rồi mới nén), đây là nén trực tiếp đúng 1 file,
//! giống hành vi lệnh `gzip`/`bzip2`/`zstd` gốc trên Unix. Bổ sung theo yêu cầu bám sát tính
//! năng 7-Zip (7-Zip cũng tạo/mở được các định dạng đơn lẻ này ngoài việc gói trong tar).
//! Cả 3 codec (flate2/bzip2/zstd) đã là dependency sẵn có (dùng cho giải nén `.tar.*`), nên
//! không cần thêm crate mới.

use crate::{CompressionLevel, Error, Format, Result};
use bzip2::read::BzDecoder;
use bzip2::write::BzEncoder;
use bzip2::Compression as BzCompression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression as GzCompression;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

/// `.xz` (`lzma-rs`) — cố tình chọn crate **thuần Rust**, không dùng `xz2`/liblzma qua FFI:
/// dự án đã một lần bị "đắng" vì phụ thuộc C library khi cross-compile Android
/// (`unrar-ng-sys`, xem `vendor/unrar-ng-sys`) nên với `.xz` — không có ràng buộc giấy phép
/// nào buộc phải dùng liblzma — chọn ngay pure-Rust để không lặp lại rủi ro đó trên
/// Android/iOS sau này. Đánh đổi: `lzma-rs` không cho chỉnh mức nén, nên tham số `level`
/// (Fast/Normal/Ultra) KHÔNG có tác dụng với `.xz` — khác các định dạng còn lại trong file
/// này, đã ghi rõ trong CLAUDE.md, không phải thiếu sót ngầm. API của `lzma-rs` cũng khác:
/// `xz_compress`/`xz_decompress` xử lý toàn bộ luồng trong 1 lệnh gọi (nhận `BufRead`+
/// `Write` trực tiếp) thay vì cho phép bọc thành 1 `Read` để đọc dần như `flate2`/`bzip2`/
/// `zstd` — vẫn không cần load hết dữ liệu vào RAM (xử lý theo khối bên trong), chỉ là
/// không lồng được vào kiểu `Box<dyn Read>` dùng chung cho 3 định dạng kia.
struct CountingSink {
    count: u64,
}

impl io::Write for CountingSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.count += buf.len() as u64;
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn gz_level(level: CompressionLevel) -> u32 {
    match level {
        CompressionLevel::Fast => 1,
        CompressionLevel::Normal => 6,
        CompressionLevel::Ultra => 9,
    }
}

fn bz2_level(level: CompressionLevel) -> u32 {
    gz_level(level) // cùng thang 1-9
}

fn zst_level(level: CompressionLevel) -> i32 {
    match level {
        CompressionLevel::Fast => 1,
        CompressionLevel::Normal => 9,
        // zstd cho phép tới 22 nhưng các mức trên ~19 chậm không tương xứng lợi ích thêm.
        CompressionLevel::Ultra => 19,
    }
}

/// `.gz`/`.bz2`/`.zst` chỉ nén được đúng 1 file (không phải container nhiều file như
/// zip/7z/tar) — khớp đúng hành vi `gzip`/`bzip2`/`zstd` gốc, không tự động gộp thư mục.
fn single_source(sources: &[PathBuf], format: Format) -> Result<&PathBuf> {
    match sources {
        [one] if one.is_file() => Ok(one),
        [one] if one.is_dir() => Err(Error::Archive(format!(
            "{format:?} chỉ nén được 1 file, không nén được thư mục — dùng .zip/.7z/.tar.gz cho thư mục"
        ))),
        _ => Err(Error::Archive(format!(
            "{format:?} chỉ nén được đúng 1 file, không nén được nhiều nguồn cùng lúc"
        ))),
    }
}

pub fn compress(sources: &[PathBuf], dest: &Path, format: Format, level: CompressionLevel) -> Result<()> {
    let source = single_source(sources, format)?;

    match format {
        Format::Gz => {
            let mut input = File::open(source).map_err(|e| Error::io(source, e))?;
            let output = File::create(dest).map_err(|e| Error::io(dest, e))?;
            let mut encoder = GzEncoder::new(output, GzCompression::new(gz_level(level)));
            io::copy(&mut input, &mut encoder).map_err(|e| Error::io(source, e))?;
            encoder.finish().map_err(|e| Error::io(dest, e))?;
        }
        Format::Bz2 => {
            let mut input = File::open(source).map_err(|e| Error::io(source, e))?;
            let output = File::create(dest).map_err(|e| Error::io(dest, e))?;
            let mut encoder = BzEncoder::new(output, BzCompression::new(bz2_level(level)));
            io::copy(&mut input, &mut encoder).map_err(|e| Error::io(source, e))?;
            encoder.finish().map_err(|e| Error::io(dest, e))?;
        }
        Format::Zst => {
            let mut input = File::open(source).map_err(|e| Error::io(source, e))?;
            let output = File::create(dest).map_err(|e| Error::io(dest, e))?;
            let mut encoder = zstd::stream::write::Encoder::new(output, zst_level(level))
                .map_err(|e| Error::io(dest, e))?;
            io::copy(&mut input, &mut encoder).map_err(|e| Error::io(source, e))?;
            encoder.finish().map_err(|e| Error::io(dest, e))?;
        }
        Format::Xz => {
            let mut input = BufReader::new(File::open(source).map_err(|e| Error::io(source, e))?);
            let mut output = File::create(dest).map_err(|e| Error::io(dest, e))?;
            lzma_rs::xz_compress(&mut input, &mut output).map_err(|e| Error::io(dest, e))?;
        }
        other => return Err(Error::UnsupportedOperation(other)),
    }
    Ok(())
}

/// Tên file sau khi giải nén: bỏ đúng đuôi mở rộng của định dạng nén (vd `data.txt.gz` ->
/// `data.txt`); nếu file nén không có đuôi tương ứng (hiếm, người dùng tự đổi tên), giữ
/// nguyên tên gốc kèm hậu tố `.out` để không ghi đè nhầm.
fn decompressed_name(archive_path: &Path, ext: &str) -> String {
    let name = archive_path.file_name().and_then(|n| n.to_str()).unwrap_or("output");
    name.strip_suffix(ext).map(str::to_string).unwrap_or_else(|| format!("{name}.out"))
}

fn ext_for(format: Format) -> Result<&'static str> {
    match format {
        Format::Gz => Ok(".gz"),
        Format::Bz2 => Ok(".bz2"),
        Format::Zst => Ok(".zst"),
        Format::Xz => Ok(".xz"),
        other => Err(Error::UnsupportedOperation(other)),
    }
}

fn open_reader(archive_path: &Path, format: Format) -> Result<Box<dyn Read>> {
    let file = File::open(archive_path).map_err(|e| Error::io(archive_path, e))?;
    let reader: Box<dyn Read> = match format {
        Format::Gz => Box::new(GzDecoder::new(file)),
        Format::Bz2 => Box::new(BzDecoder::new(file)),
        Format::Zst => {
            Box::new(zstd::stream::read::Decoder::new(file).map_err(|e| Error::io(archive_path, e))?)
        }
        // Format::Xz không lồng vào đây được — `lzma_rs::xz_decompress` xử lý cả luồng
        // trong 1 lệnh gọi (nhận `Write` đích trực tiếp), không cho bọc thành `Read` để
        // đọc dần. Các hàm extract/list_entries/test_integrity xử lý Xz riêng bên dưới.
        other => return Err(Error::UnsupportedOperation(other)),
    };
    Ok(reader)
}

pub fn extract(archive_path: &Path, dest_dir: &Path, format: Format) -> Result<()> {
    let ext = ext_for(format)?;
    std::fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;
    let out_path = dest_dir.join(decompressed_name(archive_path, ext));
    let mut out = File::create(&out_path).map_err(|e| Error::io(&out_path, e))?;

    if format == Format::Xz {
        let mut input =
            BufReader::new(File::open(archive_path).map_err(|e| Error::io(archive_path, e))?);
        lzma_rs::xz_decompress(&mut input, &mut out).map_err(|e| Error::Archive(e.to_string()))?;
    } else {
        let mut reader = open_reader(archive_path, format)?;
        io::copy(&mut reader, &mut out).map_err(|e| Error::io(archive_path, e))?;
    }
    Ok(())
}

/// FR-12 tương đương cho định dạng đơn file: chỉ có đúng 1 "entry" — kích thước là kích
/// thước SAU khi giải nén (phải giải nén thật để biết chính xác, không dùng trailer ISIZE
/// của gzip vì trường đó chỉ 32-bit, sai với file gốc > 4GB).
pub fn list_entries(archive_path: &Path, format: Format) -> Result<Vec<crate::EntryInfo>> {
    let ext = ext_for(format)?;

    let size = if format == Format::Xz {
        let mut input =
            BufReader::new(File::open(archive_path).map_err(|e| Error::io(archive_path, e))?);
        let mut sink = CountingSink { count: 0 };
        lzma_rs::xz_decompress(&mut input, &mut sink).map_err(|e| Error::Archive(e.to_string()))?;
        sink.count
    } else {
        let mut reader = open_reader(archive_path, format)?;
        io::copy(&mut reader, &mut io::sink()).map_err(|e| Error::io(archive_path, e))?
    };

    Ok(vec![crate::EntryInfo {
        name: decompressed_name(archive_path, ext),
        size,
        is_dir: false,
    }])
}

pub fn test_integrity(archive_path: &Path, format: Format) -> Result<bool> {
    if format == Format::Xz {
        let mut input =
            BufReader::new(File::open(archive_path).map_err(|e| Error::io(archive_path, e))?);
        let mut sink = CountingSink { count: 0 };
        lzma_rs::xz_decompress(&mut input, &mut sink).map_err(|e| Error::Archive(e.to_string()))?;
    } else {
        let mut reader = open_reader(archive_path, format)?;
        io::copy(&mut reader, &mut io::sink()).map_err(|e| Error::io(archive_path, e))?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CompressOptions;

    fn roundtrip(format: Format, ext: &str) {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("noi_dung.txt");
        std::fs::write(&src, "xin chao viet nam, noi dung co dau tieng viet".repeat(100)).unwrap();

        let archive = tmp.path().join(format!("noi_dung.txt{ext}"));
        compress(&[src.clone()], &archive, format, CompressionLevel::Normal).unwrap();
        assert!(archive.exists());

        assert!(test_integrity(&archive, format).unwrap());

        let entries = list_entries(&archive, format).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "noi_dung.txt");
        assert_eq!(entries[0].size, std::fs::metadata(&src).unwrap().len());

        let dest = tmp.path().join("out");
        extract(&archive, &dest, format).unwrap();
        let content = std::fs::read_to_string(dest.join("noi_dung.txt")).unwrap();
        let original = std::fs::read_to_string(&src).unwrap();
        assert_eq!(content, original);
    }

    #[test]
    fn roundtrip_gz() {
        roundtrip(Format::Gz, ".gz");
    }

    #[test]
    fn roundtrip_bz2() {
        roundtrip(Format::Bz2, ".bz2");
    }

    #[test]
    fn roundtrip_zst() {
        roundtrip(Format::Zst, ".zst");
    }

    #[test]
    fn roundtrip_xz() {
        roundtrip(Format::Xz, ".xz");
    }

    #[test]
    fn rejects_multiple_sources() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        std::fs::write(&a, "a").unwrap();
        std::fs::write(&b, "b").unwrap();

        let err = compress(&[a, b], &tmp.path().join("out.gz"), Format::Gz, CompressionLevel::Normal)
            .unwrap_err();
        assert!(matches!(err, Error::Archive(_)));
    }

    #[test]
    fn rejects_directory_source() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("mydir");
        std::fs::create_dir_all(&dir).unwrap();

        let err = compress(&[dir], &tmp.path().join("out.gz"), Format::Gz, CompressionLevel::Normal)
            .unwrap_err();
        assert!(matches!(err, Error::Archive(_)));
    }

    /// Xác nhận qua API chung của core (không chỉ gọi trực tiếp module này), khớp cách
    /// người dùng thật sẽ gọi qua CLI/Desktop/mobile.
    #[test]
    fn works_through_public_core_api() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("qua_core.txt");
        std::fs::write(&src, "noi dung qua core API").unwrap();

        let archive = tmp.path().join("qua_core.txt.gz");
        crate::compress(&[src], &archive, &CompressOptions::default()).unwrap();

        let dest = tmp.path().join("out");
        crate::extract(&archive, &dest, None).unwrap();
        let content = std::fs::read_to_string(dest.join("qua_core.txt")).unwrap();
        assert_eq!(content, "noi dung qua core API");
    }
}
