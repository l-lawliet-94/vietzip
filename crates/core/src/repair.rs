//! FR-17: Sửa file nén bị lỗi (Repair Archive) — mức độ hỗ trợ tùy định dạng, đúng như đặc
//! tả ghi rõ ("mức độ hỗ trợ tùy định dạng"), không giả vờ hỗ trợ đều cho mọi định dạng.
//!
//! ZIP là định dạng duy nhất có kỹ thuật phục hồi thật sự an toàn với API hiện có: khi
//! central directory (bảng mục lục ở cuối file, thứ mà `ZipArchive::new` cần để mở archive)
//! bị hỏng/mất — kịch bản hỏng phổ biến nhất trong thực tế (ghi/tải bị ngắt giữa chừng SAU
//! khi dữ liệu file đã ghi xong nhưng TRƯỚC khi central directory kịp ghi, vì nó luôn nằm ở
//! cuối file) — dữ liệu từng entry riêng lẻ thường vẫn còn nguyên vẹn phía trước, mỗi entry
//! có local file header riêng (chữ ký `PK\x03\x04`) đủ để đọc lại độc lập, không cần central
//! directory. Đây chính là kỹ thuật `zip::read::read_zipfile_from_stream` cung cấp sẵn (đọc
//! tuần tự theo local header, không cần seek ngược để tra central directory) — dùng lại
//! nguyên API công khai của crate, không tự viết lại parser ZIP mức thấp (rủi ro làm hỏng
//! archive nếu parse sai, giống lý do dự án đã 2 lần từ chối hand-roll ZIP writer ở mức thấp
//! cho tính năng đa luồng, xem `zip_format.rs`).
//!
//! `.7z` không có kỹ thuật tương đương: `sevenz-rust2` không expose API đọc theo "khối" độc
//! lập với header — header của `.7z` (kể cả khi không mã hoá) mô tả cấu trúc "folder"/luồng
//! nén dùng chung giữa nhiều entry, hỏng header gần như luôn nghĩa là mất khả năng biết ranh
//! giới giữa các file. Vì vậy `.7z` chỉ được PHÁT HIỆN lỗi (mở thử bằng đường bình thường),
//! không có đường phục hồi dữ liệu — trả lỗi rõ ràng thay vì âm thầm trả về archive rỗng.
//! Các định dạng còn lại (RAR/TAR-family/CAB/CPIO/DEB/RPM/nén-1-file) không tạo/ghi được ở
//! dự án này nên "sửa" (tức là: ghi lại 1 file mới) không có ý nghĩa — trả
//! `Error::UnsupportedOperation`, cùng cách các thao tác ghi khác (`add_entries`) đã làm.
//!
//! **Giới hạn đã xác nhận qua test thật, không chỉ đọc doc crate**: entry ZIP có mã hoá
//! (AES/ZipCrypto) hoặc dùng "data descriptor" (kích thước ghi SAU dữ liệu thay vì trong
//! local header — thường gặp khi nén dạng stream) bị `zip::read::read_zipfile_from_stream`
//! từ chối đọc NGAY TỪ HEADER (`ZipFileData::from_local_block` trả lỗi trước khi tên file
//! kịp được đọc — xem `zip-8.6.0/src/types.rs`), nên kỹ thuật quét thô ở đây không có cách
//! nào biết TÊN của các entry này để liệt vào `unrecoverable` — chúng chỉ đơn giản không
//! xuất hiện ở cả `recovered` lẫn `unrecoverable`. Vì vậy tổng
//! `recovered.len() + unrecoverable.len()` có thể NHỎ HƠN số entry gốc thật của 1 archive
//! (mã hoá + hỏng) — đây là giới hạn cứng của thư viện, không phải bug, và không có gì lạ:
//! bản chất mã hoá vốn được thiết kế để CHẶN đúng kiểu quét-khôi-phục-thô này.

use crate::{CompressionLevel, Error, Format, Result};
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use zip::ZipWriter;

/// Kết quả sửa file — để lớp giao diện hiển thị chính xác cái gì phục hồi được, cái gì mất,
/// thay vì chỉ trả về "thành công/thất bại" nhị phân (dữ liệu bị mất một phần khi sửa file
/// hỏng là bình thường, không phải lỗi — người dùng cần biết chính xác mất gì).
#[derive(Debug, Clone, Default)]
pub struct RepairReport {
    /// Tên các entry phục hồi được, đã ghi vào file đích.
    pub recovered: Vec<String>,
    /// Tên các entry đọc được header (nên biết tên) nhưng không phục hồi được nội dung —
    /// dữ liệu nén/CRC hỏng. KHÔNG bao gồm entry mã hoá hoặc dùng data descriptor: những
    /// entry đó không lộ ra tên qua kỹ thuật quét thô này, xem doc module.
    pub unrecoverable: Vec<String>,
}

/// Cố sửa `archive` bị lỗi, ghi kết quả (những gì phục hồi được) vào `dest`. `password` chỉ
/// dùng để thử mở archive theo đường bình thường trước (xem `try_already_healthy`) — kỹ
/// thuật quét thô khi central directory hỏng KHÔNG thể giải mã (API streaming của crate
/// `zip` không nhận tham số mật khẩu), nên nếu archive thật sự hỏng và có mật khẩu, các entry
/// đã mã hoá sẽ nằm trong `unrecoverable`, không phải lỗi của hàm này.
pub fn repair(archive: &Path, dest: &Path, password: Option<&str>) -> Result<RepairReport> {
    let format = Format::from_path(archive).ok_or_else(|| Error::UnknownFormat(archive.to_path_buf()))?;
    match format {
        Format::Zip => repair_zip(archive, dest, password),
        Format::SevenZ => repair_sevenz_detect_only(archive, dest, password),
        other => Err(Error::UnsupportedOperation(other)),
    }
}

fn repair_zip(archive: &Path, dest: &Path, password: Option<&str>) -> Result<RepairReport> {
    if let Some(report) = try_already_healthy(archive, dest, password)? {
        return Ok(report);
    }
    repair_zip_via_local_headers(archive, dest)
}

/// Đường tắt: nếu archive thật ra vẫn mở/đọc được bình thường (central directory còn nguyên,
/// mọi entry giải nén đúng CRC), không cần quét thô lại từ đầu — chỉ copy nguyên byte sang
/// `dest` (giữ nguyên mã hoá/mật khẩu nếu có, thứ mà đường quét thô không tái tạo được) và
/// báo cáo mọi entry đều phục hồi. Trả `None` nếu archive thật sự có vấn đề, để gọi tiếp
/// đường quét thô.
fn try_already_healthy(archive: &Path, dest: &Path, password: Option<&str>) -> Result<Option<RepairReport>> {
    let entries = match crate::list_entries(archive, password) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };
    if crate::test_integrity(archive, password).is_err() {
        return Ok(None);
    }
    std::fs::copy(archive, dest).map_err(|e| Error::io(dest, e))?;
    Ok(Some(RepairReport {
        recovered: entries.into_iter().filter(|e| !e.is_dir).map(|e| e.name).collect(),
        unrecoverable: Vec::new(),
    }))
}

/// 4 byte đầu của local file header ZIP — điểm bắt đầu của mỗi entry, độc lập với central
/// directory. Xem `zip::spec::Magic::LOCAL_FILE_HEADER_SIGNATURE` (không public, nên khai
/// lại giá trị literal theo đúng đặc tả ZIP — giá trị này ổn định, thuộc chuẩn ZIP từ PKWARE).
const LOCAL_HEADER_MAGIC: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];

fn repair_zip_via_local_headers(archive: &Path, dest: &Path) -> Result<RepairReport> {
    let file = File::open(archive).map_err(|e| Error::io(archive, e))?;
    let mut reader = BufReader::new(file);

    let out_file = File::create(dest).map_err(|e| Error::io(dest, e))?;
    let mut writer = ZipWriter::new(out_file);
    let options = crate::zip_format::file_options(None, CompressionLevel::Normal);

    let mut report = RepairReport::default();

    /// Kết quả của 1 bước đọc — KHÔNG được mượn `reader` (chỉ chứa dữ liệu đã sở hữu), để
    /// việc mượn `reader` bên trong `read_zipfile_from_stream` chắc chắn kết thúc ngay sau
    /// khối lệnh tính `step` (xem chú thích dưới) thay vì bị compiler giữ mượn tới hết vòng
    /// lặp — `ZipFile` có `Drop`, nên nếu match trực tiếp trên kết quả gọi hàm, Rust coi
    /// mượn `reader` còn sống tới hết khối `loop`, xung đột với lần mượn `reader` để resync
    /// ở nhánh `Err`. Tách hẳn thành 1 khối con tự chứa mọi thao tác cần `zf` là cách chuẩn
    /// để giải quyết đúng lớp lỗi mượn (borrow) này.
    enum Step {
        Done,
        Recovered(String),
        Unrecoverable(String),
        NeedResync,
    }

    loop {
        let start = reader.stream_position().map_err(|e| Error::io(archive, e))?;
        let step = {
            match zip::read::read_zipfile_from_stream(&mut reader) {
                Ok(None) => Step::Done, // gặp chữ ký central directory hợp lệ — hết phần quét thêm được
                Ok(Some(mut zf)) => {
                    // Lưu ý: entry mã hoá/dùng data descriptor KHÔNG bao giờ tới được nhánh
                    // này — `zip` crate đã trả `Err` ngay từ header (xem doc module), rơi
                    // vào nhánh `Err(_)` dưới, không lộ tên. Mọi entry ở đây chắc chắn là
                    // plaintext với kích thước khai báo đầy đủ trong local header.
                    let name = zf.name().to_string();
                    if zf.is_dir() {
                        writer
                            .add_directory(name.clone(), options.clone())
                            .map_err(crate::zip_format::map_zip_err)?;
                        Step::Recovered(name)
                    } else {
                        match recover_entry_data(&mut zf) {
                            Ok(mut tmp) => {
                                writer
                                    .start_file(name.clone(), options.clone())
                                    .map_err(crate::zip_format::map_zip_err)?;
                                io::copy(&mut tmp, &mut writer).map_err(|e| Error::io(dest, e))?;
                                Step::Recovered(name)
                            }
                            Err(_) => Step::Unrecoverable(name),
                        }
                    }
                }
                Err(_) => Step::NeedResync,
            }
        };

        match step {
            Step::Done => break,
            Step::Recovered(name) => report.recovered.push(name),
            Step::Unrecoverable(name) => report.unrecoverable.push(name),
            Step::NeedResync => {
                let resync_from = start + 1;
                reader
                    .seek(SeekFrom::Start(resync_from))
                    .map_err(|e| Error::io(archive, e))?;
                match find_next_local_header(&mut reader).map_err(|e| Error::io(archive, e))? {
                    Some(rel_offset) => {
                        reader
                            .seek(SeekFrom::Start(resync_from + rel_offset))
                            .map_err(|e| Error::io(archive, e))?;
                    }
                    None => break, // quét hết file, không còn local header nào khác
                }
            }
        }
    }

    if report.recovered.is_empty() && report.unrecoverable.is_empty() {
        drop(writer);
        let _ = std::fs::remove_file(dest);
        return Err(Error::Archive(
            "không tìm thấy entry ZIP nào có thể phục hồi — file có thể không phải định dạng zip, hoặc hỏng hoàn toàn".to_string(),
        ));
    }

    writer.finish().map_err(crate::zip_format::map_zip_err)?;
    Ok(report)
}

/// Đọc toàn bộ nội dung 1 entry ra file tạm (không giữ trong RAM — nhất quán với nguyên tắc
/// an toàn với file >1GB đã áp dụng ở nơi khác trong dự án, vd `tar_format.rs::spool_xz_to_tar`)
/// TRƯỚC khi ghi vào archive đích, để chỉ commit khi đã chắc chắn đọc/giải nén trọn vẹn
/// (CRC32 khớp) — tránh ghi 1 entry cụt/nửa vời trông như file đầy đủ nhưng thật ra bị cắt.
fn recover_entry_data<R: Read>(zf: &mut zip::read::ZipFile<'_, R>) -> io::Result<std::fs::File> {
    let mut tmp = tempfile::tempfile()?;
    io::copy(zf, &mut tmp)?;
    tmp.flush()?;
    tmp.seek(SeekFrom::Start(0))?;
    Ok(tmp)
}

/// Từ vị trí hiện tại của `reader`, quét từng byte tìm chữ ký local file header tiếp theo
/// (`PK\x03\x04`) — kỹ thuật phục hồi tiêu chuẩn khi central directory bị hỏng: nếu bản thân
/// dữ liệu của 1 entry cũng hỏng (không chỉ header), phần còn lại của file vẫn có thể chứa
/// các entry khác nguyên vẹn ở xa hơn, tìm lại điểm bắt đầu gần nhất của entry tiếp theo.
/// Trả `Ok(None)` nếu quét hết file mà không thấy — offset trả về tính từ vị trí bắt đầu quét.
fn find_next_local_header<R: Read>(reader: &mut R) -> io::Result<Option<u64>> {
    let mut window = [0u8; 4];
    let mut filled = 0usize;
    let mut scanned: u64 = 0;
    let mut byte = [0u8; 1];
    loop {
        if reader.read(&mut byte)? == 0 {
            return Ok(None);
        }
        if filled < 4 {
            window[filled] = byte[0];
            filled += 1;
        } else {
            window.copy_within(1..4, 0);
            window[3] = byte[0];
        }
        scanned += 1;
        if filled == 4 && window == LOCAL_HEADER_MAGIC {
            return Ok(Some(scanned - 4));
        }
    }
}

/// `.7z`: không có kỹ thuật quét thô an toàn (xem doc module) — chỉ phát hiện, không sửa.
/// Nếu archive thật ra vẫn mở/đọc đúng, coi như "không cần sửa" và trả về bản copy nguyên
/// vẹn (đối xứng với `try_already_healthy` của ZIP). Nếu thật sự hỏng, báo lỗi rõ ràng thay
/// vì âm thầm trả về archive rỗng hoặc giả vờ đã sửa được.
fn repair_sevenz_detect_only(archive: &Path, dest: &Path, password: Option<&str>) -> Result<RepairReport> {
    let entries = crate::list_entries(archive, password).map_err(|_| {
        Error::Archive(
            "file .7z bị lỗi — định dạng này không có kỹ thuật phục hồi dữ liệu an toàn với thư viện đang dùng (sevenz-rust2 không đọc được cấu trúc archive khi header hỏng); chỉ ZIP được hỗ trợ sửa thật sự, xem CLAUDE.md"
                .to_string(),
        )
    })?;
    crate::test_integrity(archive, password).map_err(|_| {
        Error::Archive("file .7z mở được nhưng ít nhất 1 entry giải nén lỗi — không có cách phục hồi an toàn cho .7z".to_string())
    })?;
    std::fs::copy(archive, dest).map_err(|e| Error::io(dest, e))?;
    Ok(RepairReport {
        recovered: entries.into_iter().filter(|e| !e.is_dir).map(|e| e.name).collect(),
        unrecoverable: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compress, extract, test_integrity, CompressOptions};
    use std::fs;

    fn eocd_central_directory_offset(zip_bytes: &[u8]) -> usize {
        // EOCD (End Of Central Directory) là 22 byte cuối khi archive không có comment/ZIP64
        // — cách này đọc offset thật từ chính cấu trúc file thay vì đoán/scan chữ ký (tránh
        // trùng khớp giả nếu dữ liệu nén tình cờ chứa đúng 4 byte chữ ký central directory).
        let eocd = &zip_bytes[zip_bytes.len() - 22..];
        assert_eq!(&eocd[0..4], &[0x50, 0x4B, 0x05, 0x06], "fixture phải kết thúc bằng EOCD chuẩn, không comment/ZIP64");
        u32::from_le_bytes(eocd[16..20].try_into().unwrap()) as usize
    }

    /// Kịch bản hỏng phổ biến nhất: central directory (luôn ở cuối file) bị mất/cắt cụt, dữ
    /// liệu từng entry phía trước vẫn nguyên vẹn — phải phục hồi được TOÀN BỘ entry, không
    /// entry nào rơi vào `unrecoverable`.
    #[test]
    fn repairs_all_entries_when_central_directory_is_truncated() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        fs::create_dir_all(src_dir.join("thư mục con")).unwrap();
        fs::write(src_dir.join("hello.txt"), "xin chao viet nam").unwrap();
        fs::write(src_dir.join("thư mục con/tệp có dấu.txt"), "nội dung tiếng Việt").unwrap();

        let archive = tmp.path().join("out.zip");
        compress(&[src_dir], &archive, &CompressOptions::default()).unwrap();

        let raw = fs::read(&archive).unwrap();
        let cd_offset = eocd_central_directory_offset(&raw);
        fs::write(&archive, &raw[..cd_offset]).unwrap();

        // Xác nhận trước: archive giờ thật sự không mở bình thường được (đúng tiền đề test).
        assert!(test_integrity(&archive, None).is_err());

        let repaired = tmp.path().join("repaired.zip");
        let report = repair(&archive, &repaired, None).unwrap();

        assert!(report.recovered.iter().any(|n| n.ends_with("hello.txt")));
        assert!(report.recovered.iter().any(|n| n.contains("tệp có dấu.txt")));
        assert!(report.unrecoverable.is_empty(), "khong entry nao duoc hong: {:?}", report.unrecoverable);

        assert!(test_integrity(&repaired, None).unwrap());
        let dest = tmp.path().join("out");
        extract(&repaired, &dest, None).unwrap();
        assert_eq!(fs::read_to_string(dest.join("src/hello.txt")).unwrap(), "xin chao viet nam");
        assert_eq!(
            fs::read_to_string(dest.join("src/thư mục con/tệp có dấu.txt")).unwrap(),
            "nội dung tiếng Việt"
        );
    }

    /// 1 entry ở giữa bị hỏng dữ liệu (không phải header) — phải bỏ qua đúng entry đó (vào
    /// `unrecoverable`) và vẫn phục hồi các entry còn lại, không để 1 entry hỏng làm mất
    /// toàn bộ phần còn lại của archive.
    #[test]
    fn skips_one_corrupted_entry_and_recovers_the_rest() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        let c = tmp.path().join("c.txt");
        fs::write(&a, "noi dung a lap lai ".repeat(50)).unwrap();
        fs::write(&b, "noi dung b lap lai ".repeat(50)).unwrap();
        fs::write(&c, "noi dung c lap lai ".repeat(50)).unwrap();

        let archive = tmp.path().join("mixed.zip");
        compress(&[a, b, c], &archive, &CompressOptions::default()).unwrap();

        // Tìm đúng vùng byte dữ liệu nén của b.txt qua API đọc bình thường (data_start +
        // compressed_size), rồi phá (XOR 0xFF) toàn bộ vùng đó — chỉ hỏng dữ liệu, giữ
        // nguyên local header (tên/kích thước) và toàn bộ central directory.
        let mut raw = fs::read(&archive).unwrap();
        {
            let file = File::open(&archive).unwrap();
            let mut za = zip::ZipArchive::new(file).unwrap();
            let mut idx = None;
            for i in 0..za.len() {
                if za.by_index(i).unwrap().name() == "b.txt" {
                    idx = Some(i);
                    break;
                }
            }
            let idx = idx.expect("b.txt phai co trong fixture");
            let entry = za.by_index(idx).unwrap();
            let start = entry.data_start().unwrap() as usize;
            let len = entry.compressed_size() as usize;
            for byte in &mut raw[start..start + len] {
                *byte ^= 0xFF;
            }
        }
        fs::write(&archive, &raw).unwrap();

        // Xác nhận trước: archive giờ thật sự không toàn vẹn nữa (đúng tiền đề test).
        assert!(test_integrity(&archive, None).is_err());

        let repaired = tmp.path().join("repaired.zip");
        let report = repair(&archive, &repaired, None).unwrap();

        assert!(report.recovered.iter().any(|n| n == "a.txt"));
        assert!(report.recovered.iter().any(|n| n == "c.txt"));
        assert!(report.unrecoverable.iter().any(|n| n == "b.txt"), "b.txt phai bi danh dau khong phuc hoi duoc: {report:?}");
        assert!(!report.recovered.iter().any(|n| n == "b.txt"));

        assert!(test_integrity(&repaired, None).unwrap());
        let dest = tmp.path().join("out");
        extract(&repaired, &dest, None).unwrap();
        assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "noi dung a lap lai ".repeat(50));
        assert_eq!(fs::read_to_string(dest.join("c.txt")).unwrap(), "noi dung c lap lai ".repeat(50));
        assert!(!dest.join("b.txt").exists());
    }

    /// Archive không hỏng gì cả -> đường tắt `try_already_healthy`: copy nguyên byte, không
    /// chạy qua quét thô, báo cáo mọi entry đều "recovered".
    #[test]
    fn healthy_archive_takes_fast_path_and_reports_everything_recovered() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("hello.txt"), "xin chao viet nam").unwrap();

        let archive = tmp.path().join("out.zip");
        compress(&[src_dir], &archive, &CompressOptions::default()).unwrap();

        let repaired = tmp.path().join("repaired.zip");
        let report = repair(&archive, &repaired, None).unwrap();

        assert!(report.unrecoverable.is_empty());
        assert!(report.recovered.iter().any(|n| n.ends_with("hello.txt")));
        assert_eq!(fs::read(&archive).unwrap(), fs::read(&repaired).unwrap(), "duong healthy phai copy nguyen byte");
    }

    /// Entry có mật khẩu (AES-256) không thể giải mã qua kỹ thuật quét thô (API streaming
    /// của crate `zip` không nhận mật khẩu) — khi archive hỏng thật sự, entry mã hoá KHÔNG
    /// ĐƯỢC lộ ra trong `recovered` (không được ghi ra dữ liệu rác trông như đã phục hồi).
    /// Xác nhận qua test thật (không chỉ đọc doc crate): `zip` crate từ chối đọc entry mã
    /// hoá ngay từ header, trước cả khi biết tên — nên nó cũng không lộ ra trong
    /// `unrecoverable` (không có tên để liệt vào đó), xem doc module. Entry thư mục ("src/",
    /// không mang dữ liệu) không bị đánh dấu mã hoá nên vẫn phục hồi bình thường.
    #[test]
    fn encrypted_entries_never_leak_into_recovered_when_archive_is_damaged() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("bi-mat.txt"), "noi dung bi mat").unwrap();

        let archive = tmp.path().join("secret.zip");
        let options = CompressOptions {
            password: Some("mat-khau".to_string()),
            ..Default::default()
        };
        compress(&[src_dir], &archive, &options).unwrap();

        let raw = fs::read(&archive).unwrap();
        let cd_offset = eocd_central_directory_offset(&raw);
        fs::write(&archive, &raw[..cd_offset]).unwrap();

        // Không đưa mật khẩu vào `repair` -> đường tắt healthy-check thất bại (đúng), rơi
        // xuống quét thô.
        let repaired = tmp.path().join("repaired.zip");
        let report = repair(&archive, &repaired, None).unwrap();

        assert!(!report.recovered.iter().any(|n| n.contains("bi-mat.txt")), "report: {report:?}");
        assert!(!report.unrecoverable.iter().any(|n| n.contains("bi-mat.txt")), "report: {report:?}");
        assert!(report.recovered.iter().any(|n| n == "src/"), "entry thu muc rong khong mang du lieu, van phai phuc hoi duoc: {report:?}");

        // File dest phải là zip hợp lệ, và tuyệt đối không được chứa nội dung bí mật.
        assert!(test_integrity(&repaired, None).unwrap());
        let dest = tmp.path().join("out");
        extract(&repaired, &dest, None).unwrap();
        assert!(!dest.join("src/bi-mat.txt").exists());
    }

    /// File không phải ZIP thật (không có local header nào) -> không có gì để phục hồi, phải
    /// báo lỗi rõ ràng thay vì âm thầm tạo ra 1 file `dest` rỗng trông như đã "sửa xong".
    #[test]
    fn errors_clearly_when_nothing_recoverable_is_found() {
        let tmp = tempfile::tempdir().unwrap();
        let not_a_zip = tmp.path().join("garbage.zip");
        fs::write(&not_a_zip, b"day khong phai la file zip that su, khong co local header nao ca").unwrap();

        let dest = tmp.path().join("repaired.zip");
        let err = repair(&not_a_zip, &dest, None).unwrap_err();
        assert!(matches!(err, Error::Archive(_)));
        assert!(!dest.exists(), "khong duoc tao ra dest khi khong phuc hoi duoc gi");
    }
}
