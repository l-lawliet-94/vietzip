//! ext2/ext3/ext4 dạng file ảnh HỆ THỐNG TỆP ĐỘC LẬP (standalone filesystem image — khác hẳn
//! VHD/VHDX/VMDK, những định dạng container đĩa ảo VẪN CÒN CHỨA hệ thống tệp bên trong, xem
//! CLAUDE.md mục "Deferred" — case đó cần 2 lớp crate ghép lại, rủi ro cao hơn). Chỉ đọc/giải
//! nén, mở rộng theo yêu cầu phủ danh sách định dạng 7-Zip hỗ trợ.
//!
//! Dùng crate `ext4-view` (Apache-2.0/MIT, pure Rust, `no_std`-capable, không `unsafe` ở crate
//! chính — quan trọng vì Android vẫn còn trong roadmap, cùng lý do đã chọn `lzma-rs` thay vì
//! `xz2` ở `single_format.rs`). Tài liệu chính thức của crate xác nhận đọc được cả ext2/ext3
//! ("This crate provides read-only access to ext4 filesystems. It also works with ext2
//! filesystems"), không chỉ ext4.
//!
//! Khác `cpio`/`ar` (chỉ đọc tuần tự), `ext4-view` có API cây thư mục thật
//! (`Ext4::read_dir`/`open`/`metadata`) — `walk()` (dưới) tự đệ quy vì crate không có sẵn hàm
//! "duyệt hết cây" 1 lệnh. `open()` (trả về `ext4_view::File: Read`) được dùng để giải nén
//! kiểu streaming, CỐ TÌNH không dùng `Ext4::read()` (tải nguyên file vào `Vec<u8>` cùng lúc)
//! — nhất quán với nguyên tắc an toàn file lớn (DoD #4) đã áp dụng ở `rpm_format.rs`.
//!
//! **Giới hạn phạm vi, có chủ đích**: chỉ xử lý entry loại `Regular` (file thường) và
//! `Directory`. Bỏ qua `Symlink`/thiết bị khối/ký tự/FIFO/socket — những loại entry đặc thù
//! Unix, việc "giải nén" chúng ra Windows/hệ điều hành khác không có tương đương ý nghĩa rõ
//! ràng (tái tạo symlink thật cần quyền đặc biệt trên Windows; thiết bị/socket không phải là
//! "file" theo nghĩa người dùng phổ thông hiểu). Cùng tinh thần "CAB không có khái niệm thư
//! mục thật" đã ghi ở `cab_format.rs` — không giả vờ hỗ trợ đầy đủ ngữ nghĩa Unix.
//!
//! Đuôi mở rộng nhận diện: `.ext2`, `.ext3`, `.ext4` (rõ ràng, không dùng `.img` — đuôi đó quá
//! chung chung, trùng với rất nhiều định dạng ảnh đĩa khác, dễ nhận nhầm định dạng).

use crate::{EntryInfo, Error, Result};
use ext4_view::{Ext4, Ext4Error, FileType};
use std::fs;
use std::io;
use std::path::Path;

fn map_err(archive_path: &Path, err: Ext4Error) -> Error {
    Error::io(archive_path, io::Error::other(err.to_string()))
}

fn load(archive_path: &Path) -> Result<Ext4> {
    Ext4::load_from_path(archive_path).map_err(|e| map_err(archive_path, e))
}

/// Chuyển đường dẫn nội bộ của `ext4-view` (luôn tuyệt đối, dùng `/`) sang chuỗi tương đối
/// (bỏ dấu `/` đầu) để nối vào `dest_dir`/dùng làm tên hiển thị — khớp quy ước forward-slash
/// đã dùng xuyên suốt dự án cho `EntryInfo::name`. `.display()` tự thay byte không phải UTF-8
/// bằng `�` thay vì panic/lỗi (tên file ext lý thuyết có thể không phải UTF-8 hợp lệ).
fn relative_str(path: &ext4_view::PathBuf) -> String {
    path.display().to_string().trim_start_matches('/').to_string()
}

/// Duyệt đệ quy toàn bộ cây thư mục bắt đầu từ `/` — `ext4-view` không có sẵn hàm "duyệt hết"
/// 1 lệnh, chỉ có `read_dir` cho từng thư mục. PHẢI lọc bỏ `.`/`..` (chính `read_dir` của crate
/// này trả về cả 2 entry đó, xác nhận qua test nội bộ của crate — không lọc sẽ đệ quy vô hạn).
fn walk(
    fs: &Ext4,
    dir_path: &ext4_view::PathBuf,
    archive_path: &Path,
    visit: &mut impl FnMut(ext4_view::PathBuf, FileType, u64) -> Result<()>,
) -> Result<()> {
    for entry in fs.read_dir(dir_path.as_path()).map_err(|e| map_err(archive_path, e))? {
        let entry = entry.map_err(|e| map_err(archive_path, e))?;
        let name = entry.file_name();
        if name == "." || name == ".." {
            continue;
        }
        let file_type = entry.file_type().map_err(|e| map_err(archive_path, e))?;
        let path = entry.path();
        match file_type {
            FileType::Directory => {
                visit(path.clone(), file_type, 0)?;
                walk(fs, &path, archive_path, visit)?;
            }
            FileType::Regular => {
                let size = entry.metadata().map_err(|e| map_err(archive_path, e))?.len();
                visit(path, file_type, size)?;
            }
            _ => {} // symlink/thiet bi/fifo/socket — bo qua, xem doc module
        }
    }
    Ok(())
}

pub fn extract(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let volume = load(archive_path)?;
    fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;

    let root = ext4_view::PathBuf::new("/");
    walk(&volume, &root, archive_path, &mut |path, file_type, _size| {
        let rel = relative_str(&path);
        if rel.is_empty() {
            return Ok(());
        }
        let out_path = dest_dir.join(&rel);
        if file_type == FileType::Directory {
            fs::create_dir_all(&out_path).map_err(|e| Error::io(&out_path, e))
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
            }
            let mut src = volume.open(path.as_path()).map_err(|e| map_err(archive_path, e))?;
            let mut dst = fs::File::create(&out_path).map_err(|e| Error::io(&out_path, e))?;
            io::copy(&mut src, &mut dst).map_err(|e| Error::io(&out_path, e))?;
            Ok(())
        }
    })
}

pub fn list_entries(archive_path: &Path) -> Result<Vec<EntryInfo>> {
    let volume = load(archive_path)?;
    let mut entries = Vec::new();

    let root = ext4_view::PathBuf::new("/");
    walk(&volume, &root, archive_path, &mut |path, file_type, size| {
        let rel = relative_str(&path);
        if !rel.is_empty() {
            entries.push(EntryInfo {
                name: rel,
                size,
                is_dir: file_type == FileType::Directory,
            });
        }
        Ok(())
    })?;
    Ok(entries)
}

pub fn test_integrity(archive_path: &Path) -> Result<bool> {
    let volume = load(archive_path)?;
    let root = ext4_view::PathBuf::new("/");
    walk(&volume, &root, archive_path, &mut |path, file_type, _size| {
        if file_type == FileType::Regular {
            let mut src = volume.open(path.as_path()).map_err(|e| map_err(archive_path, e))?;
            io::copy(&mut src, &mut io::sink()).map_err(|e| Error::io(archive_path, e))?;
        }
        Ok(())
    })?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// File không phải ảnh ext thật -> phải báo lỗi rõ ràng qua đường public API, không panic,
    /// không âm thầm coi là archive rỗng. Không cần fixture ext thật để xác nhận điều này.
    #[test]
    fn errors_clearly_on_garbage_file() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("garbage.ext4");
        fs::write(&archive, b"day khong phai la 1 anh ext2/ext3/ext4 that su").unwrap();

        let err = crate::list_entries(&archive, None).unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
    }

    /// Ảnh ext3 THẬT, tải từ chính repo GitHub gốc của `ext4-view`
    /// (`nicholasbishop/ext4-view-rs`, cùng giấy phép Apache-2.0/MIT với crate — xem
    /// `tests/data/README.md`), không phải tự dựng bằng tay: khác LZH (`lzh_format.rs`), một
    /// ảnh ext2/ext3/ext4 hợp lệ có quá nhiều cấu trúc liên kết (superblock, block group
    /// descriptor, bitmap, inode table, entry thư mục...) để tự tay dựng an toàn cho 1 file
    /// test — nguy cơ tự dựng sai cao hơn hẳn header LZH đơn giản. File `.zst` nén (~53KB)
    /// được giải nén NGAY LÚC CHẠY TEST bằng chính crate `zstd` đã có sẵn trong dự án (không
    /// commit bản giải nén, đúng cách chính `ext4-view` cũng làm với fixture của nó).
    fn load_real_ext3_fixture(dest_path: &Path) {
        let compressed = include_bytes!("../tests/data/ext4view_test_disk_ext3.bin.zst");
        let raw = zstd::stream::decode_all(&compressed[..]).unwrap();
        fs::write(dest_path, &raw).unwrap();
    }

    #[test]
    fn list_extract_and_test_roundtrip_on_real_ext3_image() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("test_disk_ext3.ext3");
        load_real_ext3_fixture(&archive);

        assert!(test_integrity(&archive).unwrap());

        let entries = list_entries(&archive).unwrap();
        // Cấu trúc thật đã xác nhận: thư mục gốc có "lost+found" (thư mục dự phòng chuẩn của
        // mọi hệ thống tệp họ ext) và "medium_dir" chứa 1000 file nhỏ đặt tên theo số thứ tự,
        // nội dung mỗi file là chính số đó dạng text (vd file "656" chứa "656").
        assert!(entries.iter().any(|e| e.name == "lost+found" && e.is_dir));
        assert!(entries.iter().any(|e| e.name == "medium_dir" && e.is_dir));
        let file_656 = entries.iter().find(|e| e.name == "medium_dir/656").expect("phai co medium_dir/656");
        assert!(!file_656.is_dir);
        assert_eq!(file_656.size, 3);

        let dest = tmp.path().join("out");
        extract(&archive, &dest).unwrap();
        assert!(dest.join("lost+found").is_dir());
        let content = fs::read_to_string(dest.join("medium_dir/656")).unwrap();
        assert_eq!(content, "656");
        // Toàn bộ 1000 file trong medium_dir phải được giải nén đủ, không thiếu/thừa.
        let extracted_count = fs::read_dir(dest.join("medium_dir")).unwrap().count();
        assert_eq!(extracted_count, 1000);
    }

    /// Đối chiếu chéo: nội dung đọc qua đường streaming của `extract()` (dùng `Ext4::open` +
    /// `io::copy`) phải khớp byte-for-byte với đường đọc "nguyên khối" `Ext4::read()` của
    /// chính crate — xác nhận đường streaming tự viết ở đây không làm sai lệch dữ liệu.
    #[test]
    fn streaming_extract_matches_crate_eager_read() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("test_disk_ext3.ext3");
        load_real_ext3_fixture(&archive);

        let volume = ext4_view::Ext4::load_from_path(&archive).unwrap();
        let expected = volume.read("/medium_dir/656").unwrap();

        let dest = tmp.path().join("out");
        extract(&archive, &dest).unwrap();
        let actual = fs::read(dest.join("medium_dir/656")).unwrap();

        assert_eq!(actual, expected);
    }
}
