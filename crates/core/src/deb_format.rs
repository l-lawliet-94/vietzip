//! Gói `.deb` (Debian/Ubuntu) — chỉ giải nén, mở rộng theo yêu cầu người dùng phủ toàn bộ
//! danh sách định dạng 7-Zip hỗ trợ (xem CLAUDE.md mục "7-Zip feature parity").
//!
//! `.deb` **không phải** 1 định dạng nén riêng — nó là 1 archive kiểu Unix `ar` cổ điển chứa
//! đúng 3 thành viên: `debian-binary` (số phiên bản định dạng, luôn `2.0\n`), `control.tar.*`
//! (script cài đặt + metadata gói) và `data.tar.*` (toàn bộ file thật sự sẽ được cài lên hệ
//! thống — đây mới là thứ người dùng thường muốn khi "giải nén" 1 file `.deb`). Vì vậy module
//! này không tự viết logic giải nén mới: dùng crate `ar` (MIT, cùng tác giả với `cab`/`msi`)
//! để mở container `ar`, tìm thành viên `data.tar.*`, chép ra 1 file tạm giữ đúng tên/đuôi
//! (`.tar.gz`/`.tar.xz`/`.tar.zst`/`.tar.bz2`/`.tar` tuỳ gói) rồi giao thẳng cho
//! `tar_format.rs` đã có sẵn và đã kiểm chứng — cùng kỹ thuật "spool ra file tạm" đã dùng cho
//! `.tar.xz` và `convert()`. Bỏ qua `debian-binary`/`control.tar.*` (kịch bản cài đặt/metadata
//! gói, không phải nội dung người dùng cần) — khác với việc 7-Zip hiển thị phẳng cả 3 thành
//! viên `ar` rồi bắt người dùng tự mở tiếp `data.tar.*`, ở đây tự động đi thẳng vào nội dung
//! thật, khớp tinh thần tối giản/không bắt người dùng đoán của dự án (NFR-11).

use crate::{EntryInfo, Error, Format, Result};
use std::fs::File;
use std::io;
use std::path::Path;

/// Tìm và chép thành viên `data.tar.*` ra 1 file tạm, trả về đường dẫn file tạm đó cùng
/// `Format` tương ứng (suy từ chính tên thành viên bên trong, không đoán).
fn spool_data_tar(archive_path: &Path) -> Result<(tempfile::TempPath, Format)> {
    let file = File::open(archive_path).map_err(|e| Error::io(archive_path, e))?;
    let mut ar_archive = ar::Archive::new(file);

    while let Some(entry) = ar_archive.next_entry() {
        let mut entry = entry.map_err(|e| Error::io(archive_path, e))?;
        let name = String::from_utf8_lossy(entry.header().identifier()).into_owned();
        if !name.starts_with("data.tar") {
            continue;
        }
        let format = Format::from_path(Path::new(&name))
            .ok_or_else(|| Error::Archive(format!(".deb: không nhận diện được định dạng nén của '{name}'")))?;

        let (temp_file, temp_path) = tempfile::Builder::new()
            .suffix(&format!(".{}", name.trim_start_matches("data.")))
            .tempfile()
            .map_err(|e| Error::io(archive_path, e))?
            .into_parts();
        let mut out = temp_file;
        io::copy(&mut entry, &mut out).map_err(|e| Error::io(archive_path, e))?;
        return Ok((temp_path, format));
    }

    Err(Error::Archive(
        ".deb: không tìm thấy thành viên data.tar.* bên trong (file có thể hỏng hoặc không phải .deb hợp lệ)"
            .to_string(),
    ))
}

pub fn extract(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let (temp_path, format) = spool_data_tar(archive_path)?;
    crate::tar_format::extract(&temp_path, dest_dir, format)
}

pub fn list_entries(archive_path: &Path) -> Result<Vec<EntryInfo>> {
    let (temp_path, format) = spool_data_tar(archive_path)?;
    crate::tar_format::list_entries(&temp_path, format)
}

pub fn test_integrity(archive_path: &Path) -> Result<bool> {
    let (temp_path, format) = spool_data_tar(archive_path)?;
    crate::tar_format::test_integrity(&temp_path, format)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tạo 1 file .deb thật (đúng cấu trúc `ar` + `data.tar.gz`) bằng chính API ghi của
    /// crate `ar` cộng `tar`/`flate2` đã là dependency có sẵn — không tự nối byte tay.
    fn make_sample_deb(path: &Path) {
        let mut tar_gz_bytes = Vec::new();
        {
            let enc = flate2::write::GzEncoder::new(&mut tar_gz_bytes, flate2::Compression::default());
            let mut builder = tar::Builder::new(enc);
            let mut header = tar::Header::new_gnu();
            let data = b"noi dung file that su can cai dat";
            header.set_size(data.len() as u64);
            header.set_cksum();
            builder.append_data(&mut header, "usr/bin/hello", data.as_slice()).unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }

        let out = File::create(path).unwrap();
        let mut builder = ar::Builder::new(out);

        let debian_binary = b"2.0\n";
        builder
            .append(&ar::Header::new(b"debian-binary".to_vec(), debian_binary.len() as u64), &debian_binary[..])
            .unwrap();

        let fake_control = b"Package: test\n";
        builder
            .append(&ar::Header::new(b"control.tar.gz".to_vec(), fake_control.len() as u64), &fake_control[..])
            .unwrap();

        builder
            .append(&ar::Header::new(b"data.tar.gz".to_vec(), tar_gz_bytes.len() as u64), tar_gz_bytes.as_slice())
            .unwrap();
    }

    #[test]
    fn extracts_only_data_tar_member() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("sample.deb");
        make_sample_deb(&archive);

        let entries = list_entries(&archive).unwrap();
        assert_eq!(entries.len(), 1, "chỉ lấy nội dung data.tar.*, không lẫn control.tar.*/debian-binary");
        assert!(entries[0].name.contains("usr/bin/hello"));

        assert!(test_integrity(&archive).unwrap());

        let dest = tmp.path().join("out");
        extract(&archive, &dest).unwrap();
        assert_eq!(
            std::fs::read_to_string(dest.join("usr/bin/hello")).unwrap(),
            "noi dung file that su can cai dat"
        );
    }

    #[test]
    fn works_through_public_core_api() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("sample.deb");
        make_sample_deb(&archive);

        assert!(crate::test_integrity(&archive, None).unwrap());
        let dest = tmp.path().join("out");
        crate::extract(&archive, &dest, None).unwrap();
        assert!(dest.join("usr/bin/hello").exists());
    }
}
