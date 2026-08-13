//! `.cpio` (định dạng `newc`/SVR4) — chỉ giải nén, mở rộng theo yêu cầu người dùng phủ toàn
//! bộ danh sách định dạng 7-Zip hỗ trợ (xem CLAUDE.md mục "7-Zip feature parity").
//!
//! Dùng crate `cpio` (MIT) — chỉ hỗ trợ kiểu `newc`/SVR4 (không phải `odc`/`bin`, các kiểu cũ
//! hơn nhiều và hiếm gặp ngoài hệ thống Unix rất cũ) — đây cũng là kiểu duy nhất còn dùng phổ
//! biến thực tế (initramfs của kernel Linux hiện đại, payload thô bên trong `.rpm`). Nếu gặp
//! file `.cpio` kiểu cũ, crate sẽ báo lỗi rõ ràng (sai magic number) thay vì đọc sai âm thầm.
//!
//! API của crate là dạng tuần tự (đọc từng entry một, gọi `finish()` để lấy lại reader gốc
//! cho entry kế tiếp) — khác hẳn kiểu "mở 1 lần, truy cập ngẫu nhiên theo tên" của `zip`/`cab`,
//! nên `extract`/`list_entries`/`test_integrity` đều tự lặp bằng vòng lặp thủ công thay vì
//! gọi lại 1 hàm dùng chung.

use crate::{EntryInfo, Error, Result};
use cpio::newc::Reader;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

/// Bit-mask kiểu file trong trường `mode` (chuẩn POSIX `st_mode`, xem `ModeFileType` của
/// crate `cpio`) — chỉ cần phân biệt thư mục để tạo đúng cấu trúc cây khi giải nén, các kiểu
/// khác (symlink, device, fifo...) đều xử lý như file thường (ghi nội dung thô ra, không tạo
/// symlink/device thật — tránh cần quyền hệ thống đặc biệt trên Windows).
const S_IFMT: u32 = 0o170000;
const S_IFDIR: u32 = 0o040000;

fn is_dir_mode(mode: u32) -> bool {
    mode & S_IFMT == S_IFDIR
}

/// Duyệt tuần tự từng entry trong `.cpio`, gọi `visit(name, mode, size, reader)` cho mỗi entry
/// (trừ entry `TRAILER!!!` đánh dấu kết thúc archive — crate `cpio` tự chèn, không phải file
/// thật). `visit` có thể đọc từ `reader` nếu cần nội dung; phần chưa đọc hết sẽ tự bị bỏ qua
/// bởi `finish()` (khớp hành vi crate `cpio` đã cài sẵn, không phải tự viết lại).
fn for_each_entry(
    archive_path: &Path,
    mut visit: impl FnMut(&str, u32, u32, &mut Reader<BufReader<File>>) -> Result<()>,
) -> Result<()> {
    let file = File::open(archive_path).map_err(|e| Error::io(archive_path, e))?;
    let mut inner = BufReader::new(file);
    loop {
        let mut reader = Reader::new(inner).map_err(|e| Error::io(archive_path, e))?;
        let entry = reader.entry().clone();
        if entry.is_trailer() {
            break;
        }
        visit(entry.name(), entry.mode(), entry.file_size(), &mut reader)?;
        inner = reader.finish().map_err(|e| Error::io(archive_path, e))?;
    }
    Ok(())
}

pub fn extract(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;
    for_each_entry(archive_path, |name, mode, _size, reader| {
        let dest_path = dest_dir.join(name);
        if is_dir_mode(mode) {
            std::fs::create_dir_all(&dest_path).map_err(|e| Error::io(&dest_path, e))?;
            return Ok(());
        }
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let mut out = File::create(&dest_path).map_err(|e| Error::io(&dest_path, e))?;
        io::copy(reader, &mut out).map_err(|e| Error::io(&dest_path, e))?;
        Ok(())
    })
}

pub fn list_entries(archive_path: &Path) -> Result<Vec<EntryInfo>> {
    let mut result = Vec::new();
    for_each_entry(archive_path, |name, mode, size, _reader| {
        result.push(EntryInfo {
            name: name.to_string(),
            size: size as u64,
            is_dir: is_dir_mode(mode),
        });
        Ok(())
    })?;
    Ok(result)
}

pub fn test_integrity(archive_path: &Path) -> Result<bool> {
    for_each_entry(archive_path, |_name, mode, _size, reader| {
        if !is_dir_mode(mode) {
            io::copy(reader, &mut io::sink()).map_err(|e| Error::io(archive_path, e))?;
        }
        Ok(())
    })?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::cpio::{newc, write_cpio};

    /// Tạo 1 file .cpio thật bằng chính API ghi của crate `cpio` (`write_cpio`), khớp cách
    /// các test định dạng khác trong dự án luôn dựng dữ liệu qua 1 thư viện độc lập rồi mới
    /// đọc lại bằng module của mình, không tự nối byte tay.
    fn make_sample_cpio(path: &Path) {
        let entries = vec![
            (
                newc::Builder::new("thu-muc-con").set_mode_file_type(newc::ModeFileType::Directory),
                Vec::<u8>::new(),
            ),
            (
                newc::Builder::new("hello.txt").set_mode_file_type(newc::ModeFileType::Regular),
                b"xin chao tu cpio".to_vec(),
            ),
            (
                newc::Builder::new("thu-muc-con/nested.txt").set_mode_file_type(newc::ModeFileType::Regular),
                b"file trong thu muc con".to_vec(),
            ),
        ];
        let inputs = entries
            .into_iter()
            .map(|(builder, data)| (builder, io::Cursor::new(data)));
        let out = File::create(path).unwrap();
        write_cpio(inputs, out).unwrap();
    }

    #[test]
    fn list_extract_and_test_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("sample.cpio");
        make_sample_cpio(&archive);

        let entries = list_entries(&archive).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().any(|e| e.name == "thu-muc-con" && e.is_dir));
        assert!(entries
            .iter()
            .any(|e| e.name == "hello.txt" && !e.is_dir && e.size == b"xin chao tu cpio".len() as u64));

        assert!(test_integrity(&archive).unwrap());

        let dest = tmp.path().join("out");
        extract(&archive, &dest).unwrap();
        assert_eq!(std::fs::read_to_string(dest.join("hello.txt")).unwrap(), "xin chao tu cpio");
        assert_eq!(
            std::fs::read_to_string(dest.join("thu-muc-con/nested.txt")).unwrap(),
            "file trong thu muc con"
        );
    }

    #[test]
    fn works_through_public_core_api() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("sample.cpio");
        make_sample_cpio(&archive);

        let entries = crate::list_entries(&archive, None).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(crate::test_integrity(&archive, None).unwrap());

        let dest = tmp.path().join("out");
        crate::extract(&archive, &dest, None).unwrap();
        assert!(dest.join("hello.txt").exists());
    }
}
