//! Chia nhỏ file / ghép lại — FR-04.
//!
//! **Đã xác minh qua tài liệu thực tế (không phải đoán)**: module này chia file thành
//! nhiều phần theo byte thô (`archive.zip.001`, `archive.zip.002`, ...), đúng bằng
//! `part_size_bytes` mỗi phần. Ban đầu bị coi là "không phải multi-volume ZIP/7z thật".
//! Nhưng theo chính tài liệu/thảo luận của 7-Zip (SourceForge, xem CLAUDE.md mục "True
//! multi-volume archive format"): tính năng "Split to volumes" của chính 7-Zip cho CẢ
//! `.7z` LẪN `.zip` cũng dùng đúng kỹ thuật này — chia byte thô, không có định dạng nhị
//! phân multi-volume nào trong bản thân `.7z`/`.zip` — 7-Zip thậm chí không đọc được định
//! dạng PKZIP spanning "thật" (`.z01`/`.z02`/`.zip`) của WinZip/PKZIP. Vì vậy cơ chế ở đây
//! là tương đương thật với 7-Zip, không phải bản giả lập rút gọn. Khoảng cách UX duy nhất
//! so với 7-Zip là: 7-Zip cho mở trực tiếp phần `.001` (tự ghép ngầm), còn ở đây trước kia
//! bắt người dùng tự bấm "Ghép file..." trước — `resolve_if_split` (dưới) lấp đúng chỗ đó,
//! được gọi tự động từ `extract`/`list_entries`/`test_integrity` ở `lib.rs`.

use crate::{Error, Result};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// Số phần tối đa (999) — đủ lớn cho mọi nhu cầu thực tế, giữ định dạng số 3 chữ số
/// (`.001`..`.999`) khớp quy ước split archive quen thuộc (7-Zip cũng dùng kiểu này).
const MAX_PARTS: u32 = 999;

fn part_path(base: &Path, index: u32) -> PathBuf {
    let mut name = base.as_os_str().to_os_string();
    name.push(format!(".{index:03}"));
    PathBuf::from(name)
}

/// Chia `path` thành nhiều phần tối đa `part_size_bytes` mỗi phần, đặt cạnh file gốc với
/// hậu tố `.001`, `.002`, ... Trả về danh sách đường dẫn các phần theo đúng thứ tự.
/// File gốc không bị xóa hay thay đổi — gọi thêm `std::fs::remove_file` nếu muốn chỉ giữ
/// lại các phần.
pub fn split_file(path: &Path, part_size_bytes: u64) -> Result<Vec<PathBuf>> {
    if part_size_bytes == 0 {
        return Err(Error::Archive(
            "kích thước mỗi phần phải lớn hơn 0".to_string(),
        ));
    }

    let mut input = File::open(path).map_err(|e| Error::io(path, e))?;
    let buf_size = part_size_bytes.min(1024 * 1024) as usize;
    let mut buf = vec![0u8; buf_size.max(1)];
    let mut parts = Vec::new();
    let mut index = 1u32;

    loop {
        if index > MAX_PARTS {
            return Err(Error::Archive(format!(
                "file quá lớn để chia với kích thước phần đã chọn (vượt quá {MAX_PARTS} phần)"
            )));
        }
        let out_path = part_path(path, index);
        let mut out = File::create(&out_path).map_err(|e| Error::io(&out_path, e))?;
        let mut written_this_part = 0u64;

        while written_this_part < part_size_bytes {
            let to_read = buf.len().min((part_size_bytes - written_this_part) as usize);
            let n = input
                .read(&mut buf[..to_read])
                .map_err(|e| Error::io(path, e))?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])
                .map_err(|e| Error::io(&out_path, e))?;
            written_this_part += n as u64;
        }

        if written_this_part == 0 {
            drop(out);
            let _ = std::fs::remove_file(&out_path);
            break;
        }
        parts.push(out_path);
        index += 1;
    }

    if parts.is_empty() {
        return Err(Error::Archive("file nguồn rỗng, không có gì để chia".to_string()));
    }
    Ok(parts)
}

/// Ghép các phần lại thành `dest`, bắt đầu từ `first_part` (ví dụ `archive.zip.001`) và
/// tự tìm các phần tiếp theo cùng tên gốc (`.002`, `.003`, ...) cho đến khi hết.
pub fn join_parts(first_part: &Path, dest: &Path) -> Result<()> {
    let name = first_part
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::Archive("tên phần không hợp lệ".to_string()))?;

    let Some(stripped) = name.strip_suffix(".001") else {
        return Err(Error::Archive(
            "phải chọn phần đầu tiên (đuôi .001) để ghép lại".to_string(),
        ));
    };
    let base = first_part.with_file_name(stripped);

    let mut out = File::create(dest).map_err(|e| Error::io(dest, e))?;
    let mut index = 1u32;
    let mut joined_any = false;
    loop {
        let part = part_path(&base, index);
        if !part.exists() {
            break;
        }
        let mut input = File::open(&part).map_err(|e| Error::io(&part, e))?;
        io::copy(&mut input, &mut out).map_err(|e| Error::io(&part, e))?;
        joined_any = true;
        index += 1;
    }

    if !joined_any {
        return Err(Error::Archive(format!(
            "không tìm thấy phần nào để ghép (đã tìm {})",
            part_path(&base, 1).display()
        )));
    }
    Ok(())
}

/// `true` nếu `path` là phần đầu tiên (`.001`) của 1 file đã chia — dùng để tự động ghép
/// khi người dùng mở thẳng phần `.001` (khớp hành vi thật của 7-Zip, xem doc comment đầu file).
pub fn is_first_part(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("001")
}

/// Nếu `path` là phần `.001`, ghép toàn bộ các phần vào 1 file tạm — đặt tên theo đúng phần
/// đã bỏ hậu tố `.001` (`file_stem()`, vd `archive.zip.001` -> `archive.zip`) để
/// `Format::from_path` ở `lib.rs` vẫn nhận diện đúng định dạng bên trong — rồi trả về
/// đường dẫn file tạm đó. Trả `None` nếu `path` không phải phần `.001` (không có gì cần
/// ghép). Gọi bởi `extract`/`list_entries`/`test_integrity`; người gọi chịu trách nhiệm xoá
/// file tạm sau khi dùng xong.
pub fn resolve_if_split(path: &Path) -> Result<Option<PathBuf>> {
    if !is_first_part(path) {
        return Ok(None);
    }
    let stripped_name = path
        .file_stem()
        .ok_or_else(|| Error::Archive("tên phần không hợp lệ".to_string()))?;
    let tmp = std::env::temp_dir().join(format!(
        "vietzip-joined-{}-{}",
        std::process::id(),
        stripped_name.to_string_lossy()
    ));
    join_parts(path, &tmp)?;
    Ok(Some(tmp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::Hasher;

    fn hash_file(path: &Path) -> u64 {
        let mut file = File::open(path).unwrap();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = file.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            hasher.write(&buf[..n]);
        }
        hasher.finish()
    }

    #[test]
    fn split_then_join_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("archive.zip");
        // 250KB dữ liệu không lặp đơn điệu, đủ lớn để chia ra > 1 phần với part_size nhỏ.
        let mut data = Vec::with_capacity(250_000);
        let mut seed: u32 = 42;
        while data.len() < 250_000 {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            data.push((seed >> 16) as u8);
        }
        std::fs::write(&src, &data).unwrap();

        let part_size = 100_000u64; // 250KB / 100KB -> 3 phần (100K, 100K, 50K)
        let parts = split_file(&src, part_size).unwrap();
        assert_eq!(parts.len(), 3, "phải chia thành đúng 3 phần");
        assert!(parts[0].to_string_lossy().ends_with(".001"));
        assert!(parts[1].to_string_lossy().ends_with(".002"));
        assert!(parts[2].to_string_lossy().ends_with(".003"));
        assert_eq!(std::fs::metadata(&parts[0]).unwrap().len(), 100_000);
        assert_eq!(std::fs::metadata(&parts[1]).unwrap().len(), 100_000);
        assert_eq!(std::fs::metadata(&parts[2]).unwrap().len(), 50_000);

        let rejoined = tmp.path().join("rejoined.zip");
        join_parts(&parts[0], &rejoined).unwrap();
        assert_eq!(
            hash_file(&src),
            hash_file(&rejoined),
            "file ghép lại phải giống hệt byte-for-byte file gốc"
        );
    }

    #[test]
    fn join_requires_first_part() {
        let tmp = tempfile::tempdir().unwrap();
        let not_first = tmp.path().join("archive.zip.002");
        std::fs::write(&not_first, b"x").unwrap();
        let err = join_parts(&not_first, &tmp.path().join("out.zip")).unwrap_err();
        assert!(matches!(err, Error::Archive(_)));
    }

    #[test]
    fn split_and_extract_end_to_end() {
        // Xác nhận toàn bộ pipeline thật: nén -> chia -> ghép -> giải nén -> so nội dung.
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("hello.txt"), b"xin chao viet nam").unwrap();

        let archive = tmp.path().join("out.zip");
        crate::compress(&[src_dir], &archive, &crate::CompressOptions::default()).unwrap();

        let parts = split_file(&archive, 50).unwrap();
        assert!(parts.len() > 1, "file test phải đủ lớn để chia hơn 1 phần với part_size=50");

        let rejoined = tmp.path().join("rejoined.zip");
        join_parts(&parts[0], &rejoined).unwrap();

        let dest_dir = tmp.path().join("extracted");
        crate::extract(&rejoined, &dest_dir, None).unwrap();
        let content = std::fs::read_to_string(dest_dir.join("src/hello.txt")).unwrap();
        assert_eq!(content, "xin chao viet nam");
    }

    #[test]
    fn resolve_if_split_joins_transparently() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("hello.txt"), b"noi dung khop 7-zip that").unwrap();

        let archive = tmp.path().join("out.zip");
        crate::compress(&[src_dir], &archive, &crate::CompressOptions::default()).unwrap();

        let parts = split_file(&archive, 50).unwrap();
        assert!(parts.len() > 1);

        // Khớp hành vi thật của 7-Zip: mở thẳng phần .001 (không tự tay "Ghép file..." trước).
        let resolved = resolve_if_split(&parts[0]).unwrap().expect("phải nhận diện được phần .001");
        assert!(resolved.to_string_lossy().ends_with(".zip"), "file tạm phải giữ đúng đuôi .zip để nhận diện định dạng");

        let dest_dir = tmp.path().join("extracted");
        crate::extract(&resolved, &dest_dir, None).unwrap();
        let content = std::fs::read_to_string(dest_dir.join("src/hello.txt")).unwrap();
        assert_eq!(content, "noi dung khop 7-zip that");

        std::fs::remove_file(&resolved).unwrap();
    }

    #[test]
    fn resolve_if_split_returns_none_for_non_split_path() {
        let tmp = tempfile::tempdir().unwrap();
        let plain = tmp.path().join("archive.zip");
        std::fs::write(&plain, b"x").unwrap();
        assert!(resolve_if_split(&plain).unwrap().is_none());
    }

    #[test]
    fn extract_directly_from_first_part_without_manual_join() {
        // Đây chính là hành vi mới thêm ở lib.rs::extract — kiểm tra qua public API, không
        // chỉ qua resolve_if_split trực tiếp, để chắc chắn phần wiring thật cũng hoạt động.
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("a.txt"), b"a").unwrap();

        let archive = tmp.path().join("out.7z");
        crate::compress(&[src_dir], &archive, &crate::CompressOptions::default()).unwrap();
        let parts = split_file(&archive, 40).unwrap();
        assert!(parts.len() > 1);

        let dest_dir = tmp.path().join("extracted");
        crate::extract(&parts[0], &dest_dir, None).unwrap();
        assert_eq!(std::fs::read_to_string(dest_dir.join("src/a.txt")).unwrap(), "a");

        assert!(crate::list_entries(&parts[0], None).unwrap().len() >= 1);
        assert!(crate::test_integrity(&parts[0], None).unwrap());
    }
}
