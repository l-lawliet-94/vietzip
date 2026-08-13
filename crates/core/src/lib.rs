//! Core engine dùng chung cho mọi nền tảng (CLI, Desktop, Android).
//! Xem ke-hoach-mvp.md — phạm vi MVP: nén ZIP/7Z, giải nén ZIP/7Z/TAR-family/RAR(chỉ đọc).

mod arj_format;
mod benchmark;
mod cab_format;
mod checksum;
mod chm_format;
mod cpio_format;
mod deb_format;
mod error;
mod ext_format;
mod lzh_format;
mod nsis_format;
mod rar_format;
mod repair;
mod rpm_format;
mod sevenz_format;
mod sfx;
mod single_format;
mod split;
mod tar_format;
mod udf_format;
mod zip_format;

pub use benchmark::{run_benchmark, BenchmarkResult};
pub use checksum::{compute_checksum, FileChecksum};
pub use error::{Error, Result};
pub use repair::{repair, RepairReport};
pub use sfx::{extract_embedded_archive, write_sfx, SfxInfo};
pub use split::{join_parts, split_file};

use std::path::{Path, PathBuf};

/// Định dạng file nén mà core engine nhận diện được.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Zip,
    SevenZ,
    Tar,
    TarGz,
    TarBz2,
    TarZst,
    TarXz,
    /// Chỉ giải nén — không bao giờ tạo/ghi (xem FR-02, FR-14, LICENSES.md).
    Rar,
    /// Nén 1 file đơn lẻ (không phải container nhiều file) — xem `single_format.rs`.
    Gz,
    Bz2,
    Zst,
    /// Như trên; riêng `Xz` không có tuỳ chọn mức nén (crate `lzma-rs` không hỗ trợ).
    Xz,
    /// Microsoft Cabinet — chỉ giải nén, xem `cab_format.rs`.
    Cab,
    /// `newc`/SVR4 — chỉ giải nén, xem `cpio_format.rs`.
    Cpio,
    /// Gói Debian/Ubuntu — chỉ giải nén, xem `deb_format.rs`.
    Deb,
    /// Gói Red Hat/Fedora/openSUSE — chỉ giải nén, xem `rpm_format.rs`.
    RpmPkg,
    /// LHA/LZH — chỉ giải nén, xem `lzh_format.rs`.
    Lzh,
    /// ext2/ext3/ext4 dạng file ảnh hệ thống tệp độc lập — chỉ đọc, xem `ext_format.rs`.
    Ext,
    /// ARJ (thời MS-DOS) — chỉ giải nén, xem `arj_format.rs`.
    Arj,
    /// Installer NSIS (`.exe`) — chỉ giải nén, xem `nsis_format.rs`. Trường hợp duy nhất
    /// `Format::from_path` phải đọc file thật thay vì chỉ nhìn tên (không có đuôi riêng).
    Nsis,
    /// Microsoft Compiled HTML Help — chỉ giải nén, xem `chm_format.rs`.
    Chm,
    /// Universal Disk Format (DVD/Blu-ray/đĩa quang, USB lớn) — chỉ giải nén, xem
    /// `udf_format.rs`.
    Udf,
}

impl Format {
    /// Nhận diện định dạng từ tên file (đuôi mở rộng), khớp FR-15 (nhận diện tự động).
    /// Thứ tự kiểm tra quan trọng: `.tar.gz`/`.tar.bz2`/`.tar.zst` phải khớp TRƯỚC
    /// `.gz`/`.bz2`/`.zst` đơn lẻ, nếu không `data.tar.gz` sẽ bị nhận nhầm thành `Gz`.
    ///
    /// **Ngoại lệ duy nhất, có chủ đích — `.exe`**: khác mọi nhánh khác (thuần theo tên file,
    /// không đụng tới đĩa), installer NSIS không có đuôi mở rộng riêng để phân biệt (chỉ là 1
    /// file `.exe` Windows bình thường) — không có cách nào nhận diện qua tên file. Khi gặp
    /// `.exe`, hàm này ĐỌC THẬT file để dò chữ ký `"NullsoftInst"` qua chính
    /// `nsis_format::sniff` (dùng lại `NsisInstaller::from_bytes` thật của crate, không tự cài
    /// lại việc dò chữ ký). Bất kỳ lỗi nào (không đọc được/không phải NSIS) đều rơi về `None`,
    /// giống hệt hành vi cũ trước khi định dạng này tồn tại — không phá vỡ tương thích ngược
    /// cho các `.exe` khác (SFX do chính app này tạo, hay bất kỳ chương trình Windows nào).
    /// Người dùng đã được hỏi và xác nhận đánh đổi "hàm này giờ có I/O cho 1 trường hợp" — xem
    /// CLAUDE.md mục "Broader extract-only format coverage".
    pub fn from_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?.to_ascii_lowercase();
        if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            Some(Format::TarGz)
        } else if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") {
            Some(Format::TarBz2)
        } else if name.ends_with(".tar.zst") {
            Some(Format::TarZst)
        } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
            Some(Format::TarXz)
        } else if name.ends_with(".tar") {
            Some(Format::Tar)
        } else if name.ends_with(".zip") {
            Some(Format::Zip)
        } else if name.ends_with(".7z") {
            Some(Format::SevenZ)
        } else if name.ends_with(".rar") {
            Some(Format::Rar)
        } else if name.ends_with(".gz") {
            Some(Format::Gz)
        } else if name.ends_with(".bz2") {
            Some(Format::Bz2)
        } else if name.ends_with(".zst") {
            Some(Format::Zst)
        } else if name.ends_with(".xz") {
            Some(Format::Xz)
        } else if name.ends_with(".cab") {
            Some(Format::Cab)
        } else if name.ends_with(".cpio") {
            Some(Format::Cpio)
        } else if name.ends_with(".deb") {
            Some(Format::Deb)
        } else if name.ends_with(".rpm") {
            Some(Format::RpmPkg)
        } else if name.ends_with(".lzh") || name.ends_with(".lha") {
            Some(Format::Lzh)
        } else if name.ends_with(".ext2") || name.ends_with(".ext3") || name.ends_with(".ext4") {
            Some(Format::Ext)
        } else if name.ends_with(".arj") {
            Some(Format::Arj)
        } else if name.ends_with(".exe") {
            nsis_format::sniff(path).then_some(Format::Nsis)
        } else if name.ends_with(".chm") {
            Some(Format::Chm)
        } else if name.ends_with(".udf") {
            Some(Format::Udf)
        } else {
            None
        }
    }
}

/// Một mục bên trong file nén — dùng cho FR-12 (xem trước nội dung).
#[derive(Debug, Clone)]
pub struct EntryInfo {
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
}

/// Mức độ nén — FR-03. 3 mức theo đúng đặc tả gốc (Nhanh/Cân bằng/Tối đa), không theo
/// thang 6 mức của 7-Zip (Store/Fastest/Fast/Normal/Maximum/Ultra) — giữ tối giản đúng
/// tinh thần NFR-11, người dùng phổ thông không cần phân biệt 6 mức gần giống nhau.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CompressionLevel {
    /// Nén nhanh, tỷ lệ nén thấp hơn.
    Fast,
    /// Cân bằng tốc độ/tỷ lệ nén — mặc định.
    #[default]
    Normal,
    /// Nén chậm nhất, tỷ lệ nén cao nhất.
    Ultra,
}

/// Tuỳ chọn khi nén — FR-01, FR-02, FR-03, FR-05.
#[derive(Debug, Clone, Default)]
pub struct CompressOptions {
    /// Đặt mật khẩu + mã hóa AES-256 (ZIP và 7Z đều hỗ trợ).
    pub password: Option<String>,
    /// Mức độ nén — FR-03. Mặc định `Normal` nếu không chỉ định.
    pub level: CompressionLevel,
}

/// Nén danh sách file/thư mục nguồn thành 1 file lưu trữ tại `dest`.
/// Định dạng đầu ra được suy ra từ đuôi mở rộng của `dest` (FR-01, FR-02).
pub fn compress(sources: &[PathBuf], dest: &Path, options: &CompressOptions) -> Result<()> {
    let format = Format::from_path(dest).ok_or_else(|| Error::UnknownFormat(dest.to_path_buf()))?;
    match format {
        Format::Zip => zip_format::compress(sources, dest, options),
        Format::SevenZ => sevenz_format::compress(sources, dest, options),
        Format::Gz | Format::Bz2 | Format::Zst | Format::Xz => {
            single_format::compress(sources, dest, format, options.level)
        }
        other => Err(Error::UnsupportedOperation(other)),
    }
}

/// Bọc 1 thao tác chỉ-đọc (extract/list/test) để tự động ghép trước nếu `archive` là phần
/// `.001` của 1 file đã chia (FR-04 nâng cấp — khớp hành vi thật của 7-Zip khi mở thẳng
/// `.7z.001`/`.zip.001`, xem `split.rs`'s doc comment). Dọn file tạm sau khi `action` chạy
/// xong, dù thành công hay lỗi.
fn with_resolved_archive<T>(archive: &Path, action: impl FnOnce(&Path) -> Result<T>) -> Result<T> {
    match split::resolve_if_split(archive)? {
        Some(tmp) => {
            let result = action(&tmp);
            let _ = std::fs::remove_file(&tmp);
            result
        }
        None => action(archive),
    }
}

/// Giải nén toàn bộ `archive` vào thư mục `dest_dir` (FR-10, FR-11).
pub fn extract(archive: &Path, dest_dir: &Path, password: Option<&str>) -> Result<()> {
    with_resolved_archive(archive, |archive| {
        let format = Format::from_path(archive).ok_or_else(|| Error::UnknownFormat(archive.to_path_buf()))?;
        match format {
            Format::Zip => zip_format::extract(archive, dest_dir, password),
            Format::SevenZ => sevenz_format::extract(archive, dest_dir, password),
            Format::Tar | Format::TarGz | Format::TarBz2 | Format::TarZst | Format::TarXz => {
                tar_format::extract(archive, dest_dir, format)
            }
            Format::Rar => rar_format::extract(archive, dest_dir, password),
            Format::Gz | Format::Bz2 | Format::Zst | Format::Xz => single_format::extract(archive, dest_dir, format),
            Format::Cab => cab_format::extract(archive, dest_dir),
            Format::Cpio => cpio_format::extract(archive, dest_dir),
            Format::Deb => deb_format::extract(archive, dest_dir),
            Format::RpmPkg => rpm_format::extract(archive, dest_dir),
            Format::Lzh => lzh_format::extract(archive, dest_dir),
            Format::Ext => ext_format::extract(archive, dest_dir),
            Format::Arj => arj_format::extract(archive, dest_dir, password),
            Format::Nsis => nsis_format::extract(archive, dest_dir),
            Format::Chm => chm_format::extract(archive, dest_dir),
            Format::Udf => udf_format::extract(archive, dest_dir),
        }
    })
}

/// Kéo 1 dòng trong bảng nội dung ra ngoài (Explorer/Desktop) — phần "kéo ra" còn thiếu của
/// mục "Drag-and-drop" (xem CLAUDE.md). Vì hầu hết định dạng ở đây không có API đọc ngẫu
/// nhiên theo tên (vd `cpio`/`ar`/`tar` chỉ đọc tuần tự), việc viết riêng "chỉ giải nén 1
/// entry" cho từng định dạng sẽ nhân bản rất nhiều logic đã kiểm chứng — thay vào đó hàm này
/// gọi thẳng `extract()` (giải nén toàn bộ) vào 1 thư mục tạm rồi trả về đường dẫn đúng entry
/// được yêu cầu, tái dùng 100% code đã có, đổi lấy việc giải nén dư các entry khác (chấp nhận
/// được — đây là thao tác 1 lần cho 1 archive người dùng đang xem, không phải vòng lặp lớn).
///
/// Thư mục tạm **cố tình không tự xoá** (`TempDir::keep()`) — khác các chỗ dùng file tạm
/// khác trong dự án (thường xoá ngay sau khi dùng xong trong cùng 1 lời gọi hàm), vì ở đây
/// file cần tồn tại xuyên suốt thao tác kéo-thả của hệ điều hành, diễn ra ở 1 lời gọi JS
/// riêng, sau khi hàm này đã trả về — không có điểm nào trong vòng đời để chủ động dọn dẹp
/// an toàn. Đánh đổi có chủ đích, chấp nhận rác tạm nhỏ tích luỹ theo phiên làm việc (hệ điều
/// hành tự dọn thư mục temp định kỳ) — ghi rõ ở đây để không bị hiểu nhầm là rò rỉ tài nguyên.
///
/// Lớp mỏng gọi `extract_multiple_for_drag` với 1 phần tử — xem hàm đó cho trường hợp đa
/// chọn (nhiều dòng cùng lúc).
pub fn extract_for_drag(archive: &Path, entry_name: &str, password: Option<&str>) -> Result<PathBuf> {
    extract_multiple_for_drag(archive, std::slice::from_ref(&entry_name.to_string()), password)
        .map(|mut paths| paths.remove(0))
}

/// Kéo NHIỀU dòng đã chọn cùng lúc (đa chọn trong bảng entries) — cùng kỹ thuật với
/// `extract_for_drag` (giải nén toàn bộ archive 1 lần vào thư mục tạm), chỉ khác trả về
/// nhiều đường dẫn thay vì 1. Cố tình tách hàm riêng thay vì để `extract_for_drag` nhận
/// `&[String]` luôn: giữ chữ ký hàm cũ (1 tên) không đổi cho code gọi hiện có (CLI tương lai,
/// nếu có), đồng thời `extract_for_drag` giờ chỉ là 1 lớp mỏng gọi hàm này với 1 phần tử —
/// tránh lặp lại logic tạo thư mục tạm + giải nén.
pub fn extract_multiple_for_drag(
    archive: &Path,
    entry_names: &[String],
    password: Option<&str>,
) -> Result<Vec<PathBuf>> {
    let tmp_dir = tempfile::Builder::new()
        .prefix("vietzip-drag-")
        .tempdir()
        .map_err(|e| Error::io(archive, e))?
        .keep();

    extract(archive, &tmp_dir, password)?;

    entry_names
        .iter()
        .map(|entry_name| {
            let entry_path = tmp_dir.join(entry_name);
            if !entry_path.exists() {
                return Err(Error::Archive(format!(
                    "Không tìm thấy '{entry_name}' sau khi giải nén (tên entry không khớp)"
                )));
            }
            Ok(entry_path)
        })
        .collect()
}

/// Liệt kê nội dung bên trong file nén, không giải nén (FR-12).
pub fn list_entries(archive: &Path, password: Option<&str>) -> Result<Vec<EntryInfo>> {
    with_resolved_archive(archive, |archive| {
        let format = Format::from_path(archive).ok_or_else(|| Error::UnknownFormat(archive.to_path_buf()))?;
        match format {
            Format::Zip => zip_format::list_entries(archive, password),
            Format::SevenZ => sevenz_format::list_entries(archive, password),
            Format::Tar | Format::TarGz | Format::TarBz2 | Format::TarZst | Format::TarXz => {
                tar_format::list_entries(archive, format)
            }
            Format::Rar => rar_format::list_entries(archive, password),
            Format::Gz | Format::Bz2 | Format::Zst | Format::Xz => single_format::list_entries(archive, format),
            Format::Cab => cab_format::list_entries(archive),
            Format::Cpio => cpio_format::list_entries(archive),
            Format::Deb => deb_format::list_entries(archive),
            Format::RpmPkg => rpm_format::list_entries(archive),
            Format::Lzh => lzh_format::list_entries(archive),
            Format::Ext => ext_format::list_entries(archive),
            Format::Arj => arj_format::list_entries(archive, password),
            Format::Nsis => nsis_format::list_entries(archive),
            Format::Chm => chm_format::list_entries(archive),
            Format::Udf => udf_format::list_entries(archive),
        }
    })
}

/// Kiểm tra tính toàn vẹn của file nén (FR-16, "Test Archive").
pub fn test_integrity(archive: &Path, password: Option<&str>) -> Result<bool> {
    with_resolved_archive(archive, |archive| {
        let format = Format::from_path(archive).ok_or_else(|| Error::UnknownFormat(archive.to_path_buf()))?;
        match format {
            Format::Zip => zip_format::test_integrity(archive, password),
            Format::SevenZ => sevenz_format::test_integrity(archive, password),
            Format::Tar | Format::TarGz | Format::TarBz2 | Format::TarZst | Format::TarXz => {
                tar_format::test_integrity(archive, format)
            }
            Format::Rar => rar_format::test_integrity(archive, password),
            Format::Gz | Format::Bz2 | Format::Zst | Format::Xz => single_format::test_integrity(archive, format),
            Format::Cab => cab_format::test_integrity(archive),
            Format::Cpio => cpio_format::test_integrity(archive),
            Format::Deb => deb_format::test_integrity(archive),
            Format::RpmPkg => rpm_format::test_integrity(archive),
            Format::Lzh => lzh_format::test_integrity(archive),
            Format::Ext => ext_format::test_integrity(archive),
            Format::Arj => arj_format::test_integrity(archive, password),
            Format::Nsis => nsis_format::test_integrity(archive),
            Format::Chm => chm_format::test_integrity(archive),
            Format::Udf => udf_format::test_integrity(archive),
        }
    })
}

/// FR-18/19 (in-archive file manager) — chỉ hỗ trợ `.zip`, format ghi được duy nhất mà
/// thư viện `zip` cho phép sửa tại chỗ (append/xoá/đổi tên) mà không cần build lại toàn bộ
/// định dạng archive từ đầu như `.7z` (`sevenz-rust2` không có API tương đương). Các định
/// dạng khác trả `Error::UnsupportedOperation`, giống cách AES-256 chỉ áp dụng cho zip/7z.
pub fn add_entries(
    archive: &Path,
    sources: &[PathBuf],
    password: Option<&str>,
    level: CompressionLevel,
) -> Result<()> {
    match Format::from_path(archive).ok_or_else(|| Error::UnknownFormat(archive.to_path_buf()))? {
        Format::Zip => zip_format::add_entries(archive, sources, password, level),
        other => Err(Error::UnsupportedOperation(other)),
    }
}

/// FR-18/19 — xoá các entry có tên khớp `names` khỏi 1 file `.zip`.
pub fn remove_entries(archive: &Path, names: &[String]) -> Result<()> {
    match Format::from_path(archive).ok_or_else(|| Error::UnknownFormat(archive.to_path_buf()))? {
        Format::Zip => zip_format::remove_entries(archive, names),
        other => Err(Error::UnsupportedOperation(other)),
    }
}

/// FR-18/19 — đổi tên 1 entry trong file `.zip`.
pub fn rename_entry(archive: &Path, old_name: &str, new_name: &str) -> Result<()> {
    match Format::from_path(archive).ok_or_else(|| Error::UnknownFormat(archive.to_path_buf()))? {
        Format::Zip => zip_format::rename_entry(archive, old_name, new_name),
        other => Err(Error::UnsupportedOperation(other)),
    }
}

#[cfg(test)]
mod drag_tests {
    use super::*;

    #[test]
    fn extract_for_drag_returns_correct_single_file() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("a.txt"), b"noi dung a").unwrap();
        std::fs::write(src_dir.join("b.txt"), b"noi dung b, dai hon").unwrap();

        let archive = tmp.path().join("out.zip");
        compress(&[src_dir], &archive, &CompressOptions::default()).unwrap();

        let dragged = extract_for_drag(&archive, "src/b.txt", None).unwrap();
        assert_eq!(std::fs::read_to_string(&dragged).unwrap(), "noi dung b, dai hon");
        assert!(dragged.is_absolute());
    }

    #[test]
    fn extract_for_drag_errors_on_unknown_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("a.txt"), b"x").unwrap();

        let archive = tmp.path().join("out.zip");
        compress(&[src_dir], &archive, &CompressOptions::default()).unwrap();

        let err = extract_for_drag(&archive, "khong-ton-tai.txt", None).unwrap_err();
        assert!(matches!(err, Error::Archive(_)));
    }

    /// Đa chọn: giải nén nhiều entry cùng lúc chỉ 1 lần gọi `extract()` (không lặp lại việc
    /// giải nén toàn bộ archive cho từng entry), trả về đúng đường dẫn + nội dung cho mỗi entry.
    #[test]
    fn extract_multiple_for_drag_returns_all_requested_files() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("a.txt"), b"noi dung a").unwrap();
        std::fs::write(src_dir.join("b.txt"), b"noi dung b, dai hon").unwrap();
        std::fs::write(src_dir.join("c.txt"), b"noi dung c").unwrap();

        let archive = tmp.path().join("out.zip");
        compress(&[src_dir], &archive, &CompressOptions::default()).unwrap();

        let names = vec!["src/a.txt".to_string(), "src/c.txt".to_string()];
        let dragged = extract_multiple_for_drag(&archive, &names, None).unwrap();

        assert_eq!(dragged.len(), 2);
        assert_eq!(std::fs::read_to_string(&dragged[0]).unwrap(), "noi dung a");
        assert_eq!(std::fs::read_to_string(&dragged[1]).unwrap(), "noi dung c");
    }

    #[test]
    fn extract_multiple_for_drag_errors_if_any_name_is_unknown() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("a.txt"), b"x").unwrap();

        let archive = tmp.path().join("out.zip");
        compress(&[src_dir], &archive, &CompressOptions::default()).unwrap();

        let names = vec!["src/a.txt".to_string(), "khong-ton-tai.txt".to_string()];
        let err = extract_multiple_for_drag(&archive, &names, None).unwrap_err();
        assert!(matches!(err, Error::Archive(_)));
    }
}

/// Chuyển đổi giữa các định dạng nén (tương đương "Compress" trên nội dung đã giải nén của
/// 7-Zip — 7-Zip không có nút "Convert" riêng biệt, người dùng tự giải nén rồi nén lại;
/// đây là gộp 2 bước đó thành 1 lệnh). Giải nén `source` ra thư mục tạm rồi nén lại thành
/// `dest` — không có API "chuyển mã trực tiếp" giữa các định dạng archive (khác hẳn cấu
/// trúc byte với nhau), nên đây là cách an toàn duy nhất, tái dùng đúng `extract`/`compress`
/// đã kiểm chứng thay vì viết logic chuyển đổi mới.
pub fn convert(source: &Path, dest: &Path, options: &ConvertOptions) -> Result<()> {
    let tmp = tempfile::tempdir().map_err(|e| Error::io(source, e))?;
    let extract_dir = tmp.path().join("extracted");
    extract(source, &extract_dir, options.source_password.as_deref())?;

    let entries: Vec<PathBuf> = std::fs::read_dir(&extract_dir)
        .map_err(|e| Error::io(&extract_dir, e))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();

    let compress_options = CompressOptions {
        password: options.dest_password.clone(),
        level: options.level,
    };
    compress(&entries, dest, &compress_options)
}

/// Tuỳ chọn cho `convert` — mật khẩu nguồn (để giải nén) và mật khẩu đích (để nén lại) độc
/// lập với nhau vì có thể khác nhau (vd bỏ mật khẩu khi chuyển đổi, hoặc đặt mật khẩu mới).
#[derive(Debug, Clone, Default)]
pub struct ConvertOptions {
    pub source_password: Option<String>,
    pub dest_password: Option<String>,
    pub level: CompressionLevel,
}

#[cfg(test)]
mod convert_tests {
    use super::*;

    #[test]
    fn converts_zip_to_sevenz_preserving_content() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("hello.txt"), "xin chao viet nam").unwrap();

        let zip_path = tmp.path().join("out.zip");
        compress(&[src_dir], &zip_path, &CompressOptions::default()).unwrap();

        let sevenz_path = tmp.path().join("converted.7z");
        convert(&zip_path, &sevenz_path, &ConvertOptions::default()).unwrap();

        assert!(test_integrity(&sevenz_path, None).unwrap());
        let dest = tmp.path().join("out");
        extract(&sevenz_path, &dest, None).unwrap();
        let content = std::fs::read_to_string(dest.join("src/hello.txt")).unwrap();
        assert_eq!(content, "xin chao viet nam");
    }

    #[test]
    fn converts_with_different_source_and_dest_passwords() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("bi-mat.txt"), "noi dung bi mat").unwrap();

        let zip_path = tmp.path().join("secret.zip");
        let opts = CompressOptions {
            password: Some("mat-khau-cu".to_string()),
            ..Default::default()
        };
        compress(&[src_dir], &zip_path, &opts).unwrap();

        // Chuyển sang .7z, bỏ mật khẩu cũ và đặt mật khẩu mới khác hẳn.
        let sevenz_path = tmp.path().join("converted.7z");
        let convert_opts = ConvertOptions {
            source_password: Some("mat-khau-cu".to_string()),
            dest_password: Some("mat-khau-moi".to_string()),
            level: CompressionLevel::Normal,
        };
        convert(&zip_path, &sevenz_path, &convert_opts).unwrap();

        let err = extract(&sevenz_path, &tmp.path().join("wrong"), None).unwrap_err();
        assert!(matches!(err, Error::PasswordRequired));

        let dest = tmp.path().join("ok");
        extract(&sevenz_path, &dest, Some("mat-khau-moi")).unwrap();
        let content = std::fs::read_to_string(dest.join("src/bi-mat.txt")).unwrap();
        assert_eq!(content, "noi dung bi mat");
    }
}

/// FR-03: xác nhận `level` thực sự ảnh hưởng tới kết quả, không phải tham số bị bỏ qua
/// âm thầm — nén cùng 1 nội dung ở Fast và Ultra rồi so kích thước file đầu ra.
#[cfg(test)]
mod compression_level_tests {
    use super::*;

    /// Nội dung có thể nén được nhưng không lặp lại hoàn toàn đơn điệu (nếu không, mọi
    /// mức nén đều ra cùng 1 kích thước tối thiểu do khớp mẫu quá dễ, không phân biệt
    /// được Fast/Ultra).
    fn compressible_content() -> Vec<u8> {
        let mut data = Vec::with_capacity(600_000);
        let words = ["xin", "chao", "viet", "nam", "nen", "giai", "file", "du", "lieu"];
        let mut seed: u32 = 12345;
        while data.len() < 600_000 {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let word = words[(seed as usize / 65536) % words.len()];
            data.extend_from_slice(word.as_bytes());
            data.push(b' ');
        }
        data
    }

    fn compressed_size(archive_name: &str, level: CompressionLevel) -> u64 {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("data.txt");
        std::fs::write(&src, compressible_content()).unwrap();

        let archive = tmp.path().join(archive_name);
        let options = CompressOptions {
            level,
            ..Default::default()
        };
        compress(&[src], &archive, &options).unwrap();
        std::fs::metadata(&archive).unwrap().len()
    }

    #[test]
    fn zip_ultra_compresses_at_least_as_well_as_fast() {
        let fast = compressed_size("fast.zip", CompressionLevel::Fast);
        let ultra = compressed_size("ultra.zip", CompressionLevel::Ultra);
        assert!(
            ultra <= fast,
            "Ultra ({ultra} bytes) phải nén bằng hoặc tốt hơn Fast ({fast} bytes)"
        );
        assert_ne!(ultra, fast, "level phải thực sự ảnh hưởng tới kích thước file, không bị bỏ qua");
    }

    #[test]
    fn sevenz_ultra_compresses_at_least_as_well_as_fast() {
        let fast = compressed_size("fast.7z", CompressionLevel::Fast);
        let ultra = compressed_size("ultra.7z", CompressionLevel::Ultra);
        assert!(
            ultra <= fast,
            "Ultra ({ultra} bytes) phải nén bằng hoặc tốt hơn Fast ({fast} bytes)"
        );
        assert_ne!(ultra, fast, "level phải thực sự ảnh hưởng tới kích thước file, không bị bỏ qua");
    }
}

/// DoD #4 trong ke-hoach-mvp.md mục 5: "Không crash với file/thư mục lớn (>1GB)".
/// Các test khác trong crate này chỉ dùng file mẫu vài chục byte — không đủ để phát
/// hiện lỗi kiểu "đọc/ghi toàn bộ file vào RAM thay vì streaming". `#[ignore]` vì tốn
/// thời gian/đĩa (~1,2GB) — chạy rõ ràng bằng `cargo test --ignored large_file`.
#[cfg(test)]
mod large_file_tests {
    use super::*;
    use std::hash::Hasher;
    use std::io::{Read, Write};

    /// ~1,2GB, cố tình vượt ngưỡng ">1GB" của DoD #4. Dữ liệu lặp lại (không phải
    /// random) để giữ thời gian nén hợp lý — mục tiêu của test là phát hiện crash/OOM
    /// trên luồng I/O, không phải đo tỉ lệ nén, nên độ nén cao của dữ liệu lặp không
    /// làm giảm giá trị kiểm chứng.
    const LARGE_SIZE: u64 = 1_200_000_000;

    fn make_large_file(path: &Path) {
        let mut file = std::fs::File::create(path).unwrap();
        let chunk = vec![0xABu8; 1024 * 1024];
        let mut written = 0u64;
        while written < LARGE_SIZE {
            file.write_all(&chunk).unwrap();
            written += chunk.len() as u64;
        }
    }

    fn hash_file(path: &Path) -> u64 {
        let mut file = std::fs::File::open(path).unwrap();
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

    fn roundtrip(archive_name: &str) {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("big.bin");
        make_large_file(&src);

        let archive = tmp.path().join(archive_name);
        compress(&[src.clone()], &archive, &CompressOptions::default()).unwrap();

        let dest_dir = tmp.path().join("out");
        extract(&archive, &dest_dir, None).unwrap();

        let extracted = dest_dir.join("big.bin");
        assert_eq!(
            std::fs::metadata(&extracted).unwrap().len(),
            std::fs::metadata(&src).unwrap().len(),
            "kích thước file sau giải nén không khớp file gốc"
        );
        assert_eq!(
            hash_file(&src),
            hash_file(&extracted),
            "nội dung file sau giải nén không khớp file gốc"
        );
    }

    #[test]
    #[ignore]
    fn zip_roundtrip_large_file() {
        roundtrip("big.zip");
    }

    #[test]
    #[ignore]
    fn sevenz_roundtrip_large_file() {
        roundtrip("big.7z");
    }
}
