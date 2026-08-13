//! Module họ TAR — chỉ giải nén (không nén, xem FR-02 và ke-hoach-mvp.md mục 2/3).
//! Hỗ trợ: .tar, .tar.gz, .tar.bz2, .tar.zst, .tar.xz. FR-10, FR-11, FR-12, FR-16.

use crate::{EntryInfo, Error, Format, Result};
use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use tar::Archive;

/// `.tar.xz`: `lzma-rs` (xem `single_format.rs`) không có cách bọc thành `Read` để đọc dần
/// như `flate2`/`bzip2`/`zstd` — `xz_decompress` xử lý cả luồng trong 1 lệnh gọi. Giải pháp:
/// giải nén hẳn lớp `.xz` ra 1 file tạm (`tempfile::tempfile()` — tự xoá khi đóng, an toàn
/// trên cả Windows, không phải giữ 1 `TempDir` guard sống cùng vòng đời `Box<dyn Read>`),
/// rồi coi file tạm đó như 1 `.tar` thường. Tốn thêm dung lượng đĩa tạm bằng đúng kích thước
/// `.tar` chưa nén (không phải giữ trong RAM) — chấp nhận được, cùng kiểu đánh đổi
/// "spool ra đĩa tạm" đã dùng ở `convert()`.
fn spool_xz_to_tar(archive_path: &Path, file: File) -> Result<File> {
    let mut input = BufReader::new(file);
    let mut spool = tempfile::tempfile().map_err(|e| Error::io(archive_path, e))?;
    lzma_rs::xz_decompress(&mut input, &mut spool).map_err(|e| Error::Archive(e.to_string()))?;
    spool.seek(SeekFrom::Start(0)).map_err(|e| Error::io(archive_path, e))?;
    Ok(spool)
}

fn open_reader(archive_path: &Path, format: Format) -> Result<Box<dyn Read>> {
    let file = File::open(archive_path).map_err(|e| Error::io(archive_path, e))?;
    let reader: Box<dyn Read> = match format {
        Format::Tar => Box::new(file),
        Format::TarGz => Box::new(GzDecoder::new(file)),
        Format::TarBz2 => Box::new(BzDecoder::new(file)),
        Format::TarZst => Box::new(
            zstd::stream::read::Decoder::new(file).map_err(|e| Error::io(archive_path, e))?,
        ),
        Format::TarXz => Box::new(spool_xz_to_tar(archive_path, file)?),
        other => return Err(Error::UnsupportedOperation(other)),
    };
    Ok(reader)
}

pub fn extract(archive_path: &Path, dest_dir: &Path, format: Format) -> Result<()> {
    let reader = open_reader(archive_path, format)?;
    let mut archive = Archive::new(reader);
    std::fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;
    archive
        .unpack(dest_dir)
        .map_err(|e| Error::io(archive_path, e))
}

pub fn list_entries(archive_path: &Path, format: Format) -> Result<Vec<EntryInfo>> {
    let reader = open_reader(archive_path, format)?;
    let mut archive = Archive::new(reader);
    let entries = archive
        .entries()
        .map_err(|e| Error::io(archive_path, e))?;

    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| Error::io(archive_path, e))?;
        let name = entry.path().map_err(|e| Error::io(archive_path, e))?;
        result.push(EntryInfo {
            name: name.to_string_lossy().into_owned(),
            size: entry.size(),
            is_dir: entry.header().entry_type().is_dir(),
        });
    }
    Ok(result)
}

pub fn test_integrity(archive_path: &Path, format: Format) -> Result<bool> {
    let reader = open_reader(archive_path, format)?;
    let mut archive = Archive::new(reader);
    let entries = archive
        .entries()
        .map_err(|e| Error::io(archive_path, e))?;

    for entry in entries {
        let mut entry = entry.map_err(|e| Error::io(archive_path, e))?;
        io::copy(&mut entry, &mut io::sink()).map_err(|e| Error::io(archive_path, e))?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Format;

    fn make_sample_tar_gz(path: &Path) {
        let file = File::create(path).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut builder = tar::Builder::new(enc);

        let mut header = tar::Header::new_gnu();
        let data = "nội dung tiếng Việt".as_bytes();
        header.set_size(data.len() as u64);
        header.set_cksum();
        builder
            .append_data(&mut header, "tệp có dấu.txt", data)
            .unwrap();

        builder.into_inner().unwrap().finish().unwrap();
    }

    fn make_sample_tar_xz(path: &Path) {
        let mut tar_bytes = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_bytes);
            let mut header = tar::Header::new_gnu();
            let data = "nội dung tar.xz".as_bytes();
            header.set_size(data.len() as u64);
            header.set_cksum();
            builder.append_data(&mut header, "tep.txt", data).unwrap();
            builder.into_inner().unwrap();
        }

        let mut input = io::BufReader::new(tar_bytes.as_slice());
        let mut output = File::create(path).unwrap();
        lzma_rs::xz_compress(&mut input, &mut output).unwrap();
    }

    #[test]
    fn extract_and_list_tar_xz() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("sample.tar.xz");
        make_sample_tar_xz(&archive);

        let entries = list_entries(&archive, Format::TarXz).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].name.contains("tep.txt"));

        assert!(test_integrity(&archive, Format::TarXz).unwrap());

        let dest = tmp.path().join("out");
        extract(&archive, &dest, Format::TarXz).unwrap();
        let content = std::fs::read_to_string(dest.join("tep.txt")).unwrap();
        assert_eq!(content, "nội dung tar.xz");
    }

    #[test]
    fn extract_and_list_tar_gz() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("sample.tar.gz");
        make_sample_tar_gz(&archive);

        let entries = list_entries(&archive, Format::TarGz).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].name.contains("tệp có dấu.txt"));

        assert!(test_integrity(&archive, Format::TarGz).unwrap());

        let dest = tmp.path().join("out");
        extract(&archive, &dest, Format::TarGz).unwrap();
        let content = std::fs::read_to_string(dest.join("tệp có dấu.txt")).unwrap();
        assert_eq!(content, "nội dung tiếng Việt");
    }
}
