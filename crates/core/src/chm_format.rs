//! Định dạng `.chm` (Microsoft Compiled HTML Help) — chỉ giải nén, mở rộng theo yêu cầu phủ
//! danh sách định dạng 7-Zip hỗ trợ (xem CLAUDE.md mục "Broader extract-only format coverage").
//!
//! Dùng crate `libchm` (MIT, pure Rust — README của chính crate nói rõ "Pure-Rust reader",
//! đọc thẳng source xác nhận: KHÔNG có `-sys`/FFI/`extern "C"` nào, tự cài đặt giải nén LZX từ
//! đầu trong `src/lzx.rs`). **Lưu ý quan trọng đã kiểm chứng khi điều tra**: trang GitHub của
//! crate ghi mô tả repo là "Thin Rust wrapper over ChmLib" (thư viện C cũ, không maintain từ
//! 2019) — mô tả này SAI/lỗi thời so với code thật (rất có thể sót lại từ 1 phiên bản đầu chưa
//! viết lại), đã xác nhận bằng cách đọc trực tiếp `Cargo.toml` (không dependency FFI nào) và
//! `src/lib.rs`/`src/chm.rs` (logic parse ITSF/ITSP/LZX tự viết, không gọi C). Không tin theo
//! mô tả trang GitHub, tin theo code thật đã đọc.
//!
//! **Rủi ro chấp nhận có chủ đích, đã nói rõ với người dùng trước khi làm**: crate rất mới
//! (~8 tháng tuổi tính đến thời điểm thêm), 1 sao GitHub, tác giả đơn lẻ, và — khác mọi crate
//! khác dự án này từng thêm — **repo không có lấy 1 test hay fixture nào**. Không có bằng
//! chứng nào cho thấy code đã từng chạy đúng trên 1 file `.chm` thật trước khi được thêm vào
//! đây. Bù lại: dự án tự dựng fixture test (xem `tests` dưới) VÀ tự đọc kỹ toàn bộ
//! `src/format.rs`/`src/directory.rs`/`src/chm.rs` (không phải chỉ liếc qua) trước khi tin
//! dùng — cách giảm rủi ro tốt nhất có thể trong hoàn cảnh không có fixture thật để mượn.
//!
//! API tương tự `ext_format.rs`: `ChmFile::open`, `.entries(EntrySel)` (liệt kê 1 lần, khác
//! `ext4-view` không cần tự đệ quy vì CHM chỉ có 1 directory phẳng, không phải cây thư mục
//! thật), `.read(&entry)` (tải nguyên nội dung — không có API streaming, entry CHM thường nhỏ
//! — trang HTML/hình ảnh — nên chấp nhận được, không như file archive lớn).
//!
//! **Giới hạn phạm vi, có chủ đích**: chỉ liệt kê/giải nén entry loại `EntryCategory::Normal`
//! (đường dẫn bắt đầu `/`, không phải `/#`/`/$`) — bỏ qua `Special` (dữ liệu nội bộ CHM viewer,
//! vd mục lục/index tìm kiếm) và `Meta` (không bắt đầu bằng `/`, siêu dữ liệu định dạng như
//! `::DataSpace/...`) — người dùng phổ thông giải nén 1 file `.chm` muốn nội dung thật (HTML/
//! ảnh/CSS), không muốn thấy các entry nội bộ định dạng. Cùng tinh thần "CAB không có khái
//! niệm thư mục thật" đã ghi ở `cab_format.rs`.

use crate::{EntryInfo, Error, Result};
use libchm::{ChmFile, EntrySel};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

fn map_err(err: libchm::ChmError) -> Error {
    Error::Archive(err.to_string())
}

/// Chỉ nội dung thật của người dùng — xem doc module. Loại `Dir` (đường dẫn kết thúc `/`)
/// khỏi tuyển chọn không cần thiết vì CHM không lưu entry thư mục rỗng thật sự (thư mục chỉ
/// ngầm định qua đường dẫn của các file, giống RAR/TAR — không có gì để "tạo" riêng).
const SEL: EntrySel = EntrySel::NORMAL.union(EntrySel::FILES);

/// Đường dẫn CHM luôn bắt đầu bằng `/` (xem `classify_category` của chính crate) — bỏ dấu `/`
/// đầu rồi lọc component như mọi module khác, phòng path-traversal dù CHM không có khái niệm
/// `..` trong đường dẫn của chính nó (phòng vệ thêm, cùng kiểu `rpm_format.rs`/`lzh_format.rs`).
fn relative_path(chm_path: &str) -> PathBuf {
    Path::new(chm_path.trim_start_matches('/'))
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .collect()
}

pub fn extract(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let mut chm = ChmFile::open(archive_path).map_err(map_err)?;
    fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;

    let entries = chm.entries(SEL).map_err(map_err)?;
    for entry in entries {
        let rel = relative_path(&entry.path);
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out_path = dest_dir.join(&rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let content = chm.read(&entry).map_err(map_err)?;
        let mut out_file = File::create(&out_path).map_err(|e| Error::io(&out_path, e))?;
        out_file.write_all(&content).map_err(|e| Error::io(&out_path, e))?;
    }
    Ok(())
}

pub fn list_entries(archive_path: &Path) -> Result<Vec<EntryInfo>> {
    let mut chm = ChmFile::open(archive_path).map_err(map_err)?;
    let entries = chm.entries(SEL).map_err(map_err)?;
    Ok(entries
        .into_iter()
        .filter_map(|entry| {
            let rel = relative_path(&entry.path);
            if rel.as_os_str().is_empty() {
                return None;
            }
            Some(EntryInfo {
                name: rel.to_string_lossy().replace('\\', "/"),
                size: entry.length,
                is_dir: false,
            })
        })
        .collect())
}

pub fn test_integrity(archive_path: &Path) -> Result<bool> {
    let mut chm = ChmFile::open(archive_path).map_err(map_err)?;
    let entries = chm.entries(SEL).map_err(map_err)?;
    for entry in entries {
        chm.read(&entry).map_err(map_err)?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mã hoá 1 số nguyên theo "compressed word" (cword) — dạng biến-độ-dài 7-bit dùng xuyên
    /// suốt cấu trúc thư mục CHM (độ dài đường dẫn, offset, kích thước). Suy trực tiếp từ đọc
    /// `libchm::format::parse_cword` thật (không đoán mò theo tài liệu bên ngoài): mọi byte
    /// TRỪ byte cuối cùng đặt bit cao nhất (0x80, "còn tiếp"), byte cuối cùng không đặt bit đó
    /// ("byte kết thúc"). Nhóm 7-bit có ý nghĩa lớn nhất (MSB) đứng trước.
    fn encode_cword(value: u64) -> Vec<u8> {
        let mut groups = vec![(value & 0x7f) as u8];
        let mut v = value >> 7;
        while v > 0 {
            groups.push((v & 0x7f) as u8);
            v >>= 7;
        }
        groups.reverse();
        let last = groups.len() - 1;
        for (i, g) in groups.iter_mut().enumerate() {
            if i != last {
                *g |= 0x80;
            }
        }
        groups
    }

    /// Dựng 1 file `.chm` TỐI THIỂU nhưng hợp lệ: 1 header ITSF v3, 1 header ITSP với đúng 1
    /// khối thư mục PMGL (không cần khối chỉ mục PMGI — `index_root = -1` báo với reader dùng
    /// thẳng `index_head` làm khối duy nhất, xác nhận qua đọc `Directory::new` thật), và
    /// KHÔNG có mục `::DataSpace/...` nào — nghĩa là không có nén LZX, mọi entry lưu thô
    /// (`space = 0`). `libchm::chm::ChmFile::load_decompressor` tự xử lý việc thiếu các mục đó
    /// một cách an toàn (trả `Ok(None)`, xác nhận qua đọc source thật), nên bỏ qua LZX hoàn
    /// toàn là hợp lệ, không phải cắt góc — CHM không nén vẫn là CHM thật.
    ///
    /// Mọi offset trường trong header (ITSF 0x60 byte, ITSP 0x54 byte, PMGL header 0x14 byte)
    /// lấy trực tiếp từ hằng số và logic đọc thật trong `libchm::format`, không phải suy diễn
    /// từ tài liệu định dạng CHM bên ngoài — cùng nguyên tắc đã dùng cho fixture LZH ở
    /// `lzh_format.rs` (dựng fixture test cho 1 định dạng KHÔNG có writer, dựa thẳng vào chính
    /// parser thật sẽ đọc lại nó, verify bằng việc parser đó đọc đúng ngay lần thử đầu).
    fn build_minimal_chm(files: &[(&str, &[u8])]) -> Vec<u8> {
        const ITSF_LEN: usize = 0x60;
        const ITSP_LEN: usize = 0x54;
        const PMGL_HEADER_LEN: usize = 0x14;
        const BLOCK_LEN: u32 = 4096;

        // --- Khối thư mục PMGL: header + entry, phải dựng trước để biết free_space ---
        let mut entries_bytes = Vec::new();
        let mut data_bytes = Vec::new();
        for (path, content) in files {
            entries_bytes.extend(encode_cword(path.len() as u64));
            entries_bytes.extend(path.as_bytes());
            entries_bytes.extend(encode_cword(0)); // space = 0 (khong nen)
            entries_bytes.extend(encode_cword(data_bytes.len() as u64)); // start
            entries_bytes.extend(encode_cword(content.len() as u64)); // length
            data_bytes.extend_from_slice(content);
        }
        assert!(
            PMGL_HEADER_LEN + entries_bytes.len() <= BLOCK_LEN as usize,
            "fixture qua nhieu entry cho 1 khoi PMGL"
        );
        let free_space = BLOCK_LEN as usize - (PMGL_HEADER_LEN + entries_bytes.len());

        let mut pmgl = Vec::new();
        pmgl.extend_from_slice(b"PMGL");
        pmgl.write_all(&(free_space as u32).to_le_bytes()).unwrap();
        pmgl.extend_from_slice(&[0u8; 8]); // vung khong duoc parser doc toi
        pmgl.write_all(&(-1i32).to_le_bytes()).unwrap(); // block_next = -1 (khong con khoi nao khac)
        pmgl.extend_from_slice(&entries_bytes);
        pmgl.resize(BLOCK_LEN as usize, 0);

        // --- Header ITSP (0x54 byte), ngay truoc khoi PMGL ---
        let mut itsp = vec![0u8; ITSP_LEN];
        itsp[0..4].copy_from_slice(b"ITSP");
        itsp[4..8].copy_from_slice(&1u32.to_le_bytes()); // version = 1
        itsp[8..12].copy_from_slice(&(ITSP_LEN as u32).to_le_bytes()); // header_len
        itsp[0x10..0x14].copy_from_slice(&BLOCK_LEN.to_le_bytes());
        itsp[0x1c..0x20].copy_from_slice(&(-1i32).to_le_bytes()); // index_root = -1 (khong PMGI)
        itsp[0x20..0x24].copy_from_slice(&0i32.to_le_bytes()); // index_head = khoi PMGL so 0

        // --- Header ITSF v3 (0x60 byte), o dau file ---
        let dir_offset = ITSF_LEN as u64;
        let dir_len = (ITSP_LEN + BLOCK_LEN as usize) as u64;
        let data_offset = dir_offset + dir_len;

        let mut itsf = vec![0u8; ITSF_LEN];
        itsf[0..4].copy_from_slice(b"ITSF");
        itsf[4..8].copy_from_slice(&3u32.to_le_bytes()); // version = 3
        itsf[8..12].copy_from_slice(&(ITSF_LEN as u32).to_le_bytes()); // header_len
        itsf[0x48..0x50].copy_from_slice(&dir_offset.to_le_bytes());
        itsf[0x50..0x58].copy_from_slice(&dir_len.to_le_bytes());
        itsf[0x58..0x60].copy_from_slice(&data_offset.to_le_bytes());

        let mut out = itsf;
        out.extend_from_slice(&itsp);
        out.extend_from_slice(&pmgl);
        out.extend_from_slice(&data_bytes);
        out
    }

    #[test]
    fn works_through_public_core_api() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("sample.chm");
        let data = build_minimal_chm(&[
            ("/index.html", b"<html>hello chm</html>"),
            ("/about.txt", b"gioi thieu ve chm test"),
        ]);
        fs::write(&archive, &data).unwrap();

        assert!(crate::test_integrity(&archive, None).unwrap());

        let entries = crate::list_entries(&archive, None).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.name == "index.html" && e.size == b"<html>hello chm</html>".len() as u64));
        assert!(entries.iter().any(|e| e.name == "about.txt" && e.size == b"gioi thieu ve chm test".len() as u64));

        let dest = tmp.path().join("out");
        crate::extract(&archive, &dest, None).unwrap();
        assert_eq!(fs::read_to_string(dest.join("index.html")).unwrap(), "<html>hello chm</html>");
        assert_eq!(fs::read_to_string(dest.join("about.txt")).unwrap(), "gioi thieu ve chm test");
    }

    #[test]
    fn list_extract_and_test_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("sample.chm");
        let data = build_minimal_chm(&[("/index.html", b"noi dung trang chinh")]);
        fs::write(&archive, &data).unwrap();

        assert!(test_integrity(&archive).unwrap());
        let entries = list_entries(&archive).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "index.html");
        assert_eq!(entries[0].size, b"noi dung trang chinh".len() as u64);
        assert!(!entries[0].is_dir);

        let dest = tmp.path().join("out");
        extract(&archive, &dest).unwrap();
        assert_eq!(fs::read_to_string(dest.join("index.html")).unwrap(), "noi dung trang chinh");
    }

    /// File không phải CHM thật -> lỗi rõ ràng, không panic.
    #[test]
    fn errors_clearly_on_garbage_file() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("garbage.chm");
        fs::write(&archive, b"day khong phai la file CHM that su").unwrap();

        let err = crate::list_entries(&archive, None).unwrap_err();
        assert!(matches!(err, Error::Archive(_)));
    }
}
