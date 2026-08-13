//! Module ZIP — nén (Deflate, tuỳ chọn mật khẩu AES-256) và giải nén.
//! FR-01, FR-02, FR-05, FR-10, FR-11, FR-12, FR-16.

use crate::{CompressOptions, CompressionLevel, EntryInfo, Error, Result};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zip::result::ZipError;
use zip::write::FileOptions;
use zip::{AesMode, CompressionMethod, ZipArchive, ZipWriter};


pub(crate) fn map_zip_err(err: ZipError) -> Error {
    match err {
        ZipError::InvalidPassword => Error::WrongPassword,
        ZipError::UnsupportedArchive(msg) if msg == ZipError::PASSWORD_REQUIRED => {
            Error::PasswordRequired
        }
        other => Error::Archive(other.to_string()),
    }
}

/// FR-03: Deflate nhận mức nén 0-9 (mặc định 6) — không dùng Zopfli nên không có dải
/// 10-264. Fast/Normal/Ultra ánh xạ sang 1/6/9.
fn deflate_level(level: CompressionLevel) -> i64 {
    match level {
        CompressionLevel::Fast => 1,
        CompressionLevel::Normal => 6,
        CompressionLevel::Ultra => 9,
    }
}

pub(crate) fn file_options(password: Option<&str>, level: CompressionLevel) -> FileOptions<'_, ()> {
    let base = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(deflate_level(level)));
    match password {
        Some(pw) => base.with_aes_encryption(AesMode::Aes256, pw),
        None => base,
    }
}

/// Chuẩn hoá tên entry trong zip: luôn dùng `/`, không có ký tự ổ đĩa Windows.
fn zip_entry_name(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Số đơn vị (nguồn top-level, hoặc con trực tiếp của 1 thư mục) tối thiểu để đáng bỏ công
/// chia luồng — với 1 đơn vị thì không có gì để chia, với quá ít đơn vị thì overhead tạo
/// file tạm/ghép lại không đáng so với lợi ích.
const PARALLEL_MIN_UNITS: usize = 2;

pub fn compress(sources: &[PathBuf], dest: &Path, options: &CompressOptions) -> Result<()> {
    if sources.len() >= PARALLEL_MIN_UNITS {
        compress_parallel(sources, dest, options)
    } else if sources.len() == 1 && sources[0].is_dir() {
        compress_single_folder(&sources[0], dest, options)
    } else {
        compress_sequential(sources, dest, options)
    }
}

/// Mỗi đơn vị công việc: đường dẫn thật trên đĩa + đường dẫn entry mong muốn trong archive
/// (đã bao gồm mọi tiền tố thư mục cha) — cho phép các hàm nén dùng chung logic dù đơn vị
/// đến từ nhiều nguồn top-level khác nhau hay từ các con của cùng 1 thư mục.
type Unit = (PathBuf, PathBuf);

fn source_units(sources: &[PathBuf]) -> Vec<Unit> {
    sources
        .iter()
        .map(|s| {
            let base_name = s.file_name().map(PathBuf::from).unwrap_or_else(|| s.clone());
            (s.clone(), base_name)
        })
        .collect()
}

fn compress_sequential(sources: &[PathBuf], dest: &Path, options: &CompressOptions) -> Result<()> {
    compress_units_sequential(&source_units(sources), dest, options)
}

fn compress_units_sequential(units: &[Unit], dest: &Path, options: &CompressOptions) -> Result<()> {
    let file = File::create(dest).map_err(|e| Error::io(dest, e))?;
    let mut writer = ZipWriter::new(file);
    let password = options.password.as_deref();

    for (fs_path, entry_path) in units {
        add_path(&mut writer, fs_path, entry_path, password, options.level)?;
    }

    writer.finish().map_err(map_zip_err)?;
    Ok(())
}

/// Mở rộng của kỹ thuật `compress_parallel` (xem doc comment ngay dưới) sang trường hợp
/// nén ĐÚNG 1 thư mục duy nhất: trước đây case này luôn tuần tự bên trong thư mục dù có
/// hàng trăm file con, vì `compress_parallel` chỉ chia theo từng *nguồn* top-level. Ở đây
/// các con trực tiếp (`fs::read_dir`) của thư mục đó được coi như các "đơn vị" độc lập để
/// chia luồng — dùng lại nguyên `compress_parallel_units`, chỉ khác là bản thân thư mục gốc
/// (không luồng nào sở hữu nó) cần được thêm 1 lần dưới dạng entry thư mục rỗng vào archive
/// cuối trước khi ghép các phần, để khớp với những gì `add_path` tuần tự vẫn làm. Vẫn chỉ
/// chia được 1 cấp — 1 thư mục con duy nhất chứa rất nhiều file bên trong nó thì phần đó vẫn
/// tuần tự, giống hệt giới hạn "theo nguồn" gốc, chỉ dịch xuống 1 cấp.
fn compress_single_folder(dir: &Path, dest: &Path, options: &CompressOptions) -> Result<()> {
    let base_name = dir.file_name().map(PathBuf::from).unwrap_or_else(|| dir.to_path_buf());
    let mut children: Vec<_> = fs::read_dir(dir)
        .map_err(|e| Error::io(dir, e))?
        .filter_map(|e| e.ok())
        .collect();

    if children.len() < PARALLEL_MIN_UNITS {
        return compress_sequential(&[dir.to_path_buf()], dest, options);
    }

    children.sort_by_key(|e| e.file_name());
    let units: Vec<Unit> = children
        .iter()
        .map(|c| (c.path(), base_name.join(c.file_name())))
        .collect();
    compress_parallel_units(&units, &[base_name], dest, options)
}

/// "Đa luồng" tương đương tính năng multi-threading của 7-Zip, trong giới hạn API mà crate
/// `zip`/`sevenz-rust2` cho phép: cả hai đều không hỗ trợ chia nhỏ 1 file để nén song song
/// ở mức khối (block-level) như LZMA2 đa luồng thật của 7-Zip — viết lại tầng ghi ZIP mức
/// thấp để tự làm việc đó là rủi ro quá cao so với lợi ích cho dự án này. Thay vào đó, nén
/// SONG SONG TỪNG NGUỒN top-level (`sources`) độc lập vào các file `.zip` tạm riêng trên
/// nhiều luồng CPU, rồi ghép tuần tự bằng `raw_copy_file` (copy nguyên byte đã nén, không
/// nén lại lần 2 — cùng API đã dùng và test kỹ ở `rewrite_archive`). An toàn tuyệt đối vì
/// mỗi luồng ghi vào file riêng của nó, không có state dùng chung; bước ghép chỉ copy byte,
/// không có rủi ro làm hỏng archive. `compress_single_folder` (dưới) mở rộng đúng kỹ thuật
/// này sang trường hợp nén 1 thư mục duy nhất, chia theo các con trực tiếp của nó thay vì
/// theo `sources`. Giới hạn đã biết, vẫn còn nguyên: chỉ áp dụng cho ZIP (7z không có API
/// tương đương để ghép archive ở mức byte); chỉ chia được 1 cấp — 1 thư mục con duy nhất
/// chứa rất nhiều file bên trong nó thì phần đó vẫn tuần tự; và hoàn toàn không có cách nào
/// chia nhỏ 1 file đơn lẻ để nén song song ở mức khối (block-level) như LZMA2 đa luồng thật
/// của 7-Zip — đó vẫn là giới hạn cứng của API `zip`/`sevenz-rust2`, không phải thứ mở rộng
/// này giải quyết được.
fn compress_parallel(sources: &[PathBuf], dest: &Path, options: &CompressOptions) -> Result<()> {
    compress_parallel_units(&source_units(sources), &[], dest, options)
}

/// Lõi dùng chung của cả 2 đường song song hoá (nhiều nguồn top-level, và các con trực
/// tiếp của 1 thư mục): chia `units` cho các luồng CPU, mỗi luồng nén phần của mình vào 1
/// file `.zip` tạm riêng (không state dùng chung — an toàn tuyệt đối), rồi ghép tuần tự
/// bằng `raw_copy_file` (copy nguyên byte đã nén, không nén lại lần 2). `pre_directories`
/// là các entry thư mục rỗng cần ghi thẳng vào archive cuối trước khi ghép — dùng cho
/// trường hợp thư mục gốc của `compress_single_folder` mà không luồng con nào sở hữu.
fn compress_parallel_units(
    units: &[Unit],
    pre_directories: &[PathBuf],
    dest: &Path,
    options: &CompressOptions,
) -> Result<()> {
    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(units.len());

    let tmp_dir = tempfile::tempdir().map_err(|e| Error::io(dest, e))?;
    let mut chunks: Vec<Vec<Unit>> = vec![Vec::new(); worker_count];
    for (i, unit) in units.iter().enumerate() {
        chunks[i % worker_count].push(unit.clone());
    }

    let part_paths: Vec<PathBuf> = chunks
        .iter()
        .enumerate()
        .filter(|(_, chunk)| !chunk.is_empty())
        .map(|(i, _)| tmp_dir.path().join(format!("part-{i}.zip")))
        .collect();

    let results: Mutex<Vec<Result<()>>> = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (chunk, part_path) in chunks.iter().filter(|c| !c.is_empty()).zip(&part_paths) {
            let results = &results;
            scope.spawn(move || {
                let r = compress_units_sequential(chunk, part_path, options);
                results.lock().unwrap().push(r);
            });
        }
    });
    for r in results.into_inner().unwrap() {
        r?;
    }

    let out_file = File::create(dest).map_err(|e| Error::io(dest, e))?;
    let mut writer = ZipWriter::new(out_file);
    let password = options.password.as_deref();
    for dir_entry in pre_directories {
        writer
            .add_directory(zip_entry_name(dir_entry), file_options(password, options.level))
            .map_err(map_zip_err)?;
    }
    for part_path in &part_paths {
        let mut part_archive = open_archive(part_path)?;
        for i in 0..part_archive.len() {
            let entry = part_archive.by_index_raw(i).map_err(map_zip_err)?;
            writer.raw_copy_file(entry).map_err(map_zip_err)?;
        }
    }
    writer.finish().map_err(map_zip_err)?;
    Ok(())
}

fn add_path<W: Write + io::Seek>(
    writer: &mut ZipWriter<W>,
    fs_path: &Path,
    entry_path: &Path,
    password: Option<&str>,
    level: CompressionLevel,
) -> Result<()> {
    if fs_path.is_dir() {
        writer
            .add_directory(zip_entry_name(entry_path), file_options(password, level))
            .map_err(map_zip_err)?;
        let mut children: Vec<_> = fs::read_dir(fs_path)
            .map_err(|e| Error::io(fs_path, e))?
            .filter_map(|e| e.ok())
            .collect();
        children.sort_by_key(|e| e.file_name());
        for child in children {
            let child_fs_path = child.path();
            let child_entry_path = entry_path.join(child.file_name());
            add_path(writer, &child_fs_path, &child_entry_path, password, level)?;
        }
    } else {
        writer
            .start_file(zip_entry_name(entry_path), file_options(password, level))
            .map_err(map_zip_err)?;
        let mut f = File::open(fs_path).map_err(|e| Error::io(fs_path, e))?;
        io::copy(&mut f, writer).map_err(|e| Error::io(fs_path, e))?;
    }
    Ok(())
}

fn open_archive(archive: &Path) -> Result<ZipArchive<File>> {
    let file = File::open(archive).map_err(|e| Error::io(archive, e))?;
    ZipArchive::new(file).map_err(map_zip_err)
}

pub fn extract(archive_path: &Path, dest_dir: &Path, password: Option<&str>) -> Result<()> {
    let mut archive = open_archive(archive_path)?;
    fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;

    for i in 0..archive.len() {
        let mut entry = match password {
            Some(pw) => archive.by_index_decrypt(i, pw.as_bytes()),
            None => archive.by_index(i),
        }
        .map_err(map_zip_err)?;

        let Some(relative) = entry.enclosed_name() else {
            continue; // bỏ qua entry có đường dẫn không an toàn (zip-slip)
        };
        let out_path = dest_dir.join(&relative);

        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| Error::io(&out_path, e))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
            }
            let mut out_file = File::create(&out_path).map_err(|e| Error::io(&out_path, e))?;
            io::copy(&mut entry, &mut out_file).map_err(|e| Error::io(&out_path, e))?;
        }
    }
    Ok(())
}

pub fn list_entries(archive_path: &Path, password: Option<&str>) -> Result<Vec<EntryInfo>> {
    let mut archive = open_archive(archive_path)?;
    let mut entries = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let entry = match password {
            Some(pw) => archive.by_index_decrypt(i, pw.as_bytes()),
            None => archive.by_index(i),
        }
        .map_err(map_zip_err)?;
        entries.push(EntryInfo {
            name: entry.name().to_string(),
            size: entry.size(),
            is_dir: entry.is_dir(),
        });
    }
    Ok(entries)
}

/// FR-18/19 — thêm file/thư mục vào 1 file `.zip` đã có sẵn mà không nén lại từ đầu
/// (`ZipWriter::new_append` chỉ đọc/ghi lại phần cuối file, giữ nguyên các entry cũ).
pub fn add_entries(
    archive: &Path,
    sources: &[PathBuf],
    password: Option<&str>,
    level: CompressionLevel,
) -> Result<()> {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(archive)
        .map_err(|e| Error::io(archive, e))?;
    let mut writer = ZipWriter::new_append(file).map_err(map_zip_err)?;

    for source in sources {
        let base_name = source
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| source.clone());
        add_path(&mut writer, source, &base_name, password, level)?;
    }

    writer.finish().map_err(map_zip_err)?;
    Ok(())
}

/// Ghi lại toàn bộ archive vào 1 file tạm cạnh file gốc rồi thay thế — dùng chung cho
/// xoá/đổi tên entry, vì định dạng ZIP không hỗ trợ sửa tại chỗ. `visit` quyết định mỗi
/// entry gốc được bỏ qua, giữ nguyên tên, hay đổi tên khi copy sang file mới; dùng
/// `raw_copy_file*` (copy nguyên byte đã nén, không giải nén rồi nén lại) nên giữ nguyên
/// mật khẩu/mã hoá đã có mà không cần biết mật khẩu.
fn rewrite_archive(archive: &Path, mut visit: impl FnMut(&str) -> EntryAction) -> Result<()> {
    let mut src = open_archive(archive)?;

    let mut tmp_name = archive.as_os_str().to_os_string();
    tmp_name.push(".tmp");
    let tmp_path = PathBuf::from(tmp_name);

    {
        let tmp_file = File::create(&tmp_path).map_err(|e| Error::io(&tmp_path, e))?;
        let mut writer = ZipWriter::new(tmp_file);
        for i in 0..src.len() {
            let file = src.by_index_raw(i).map_err(map_zip_err)?;
            match visit(file.name()) {
                EntryAction::Skip => {}
                EntryAction::Keep => writer.raw_copy_file(file).map_err(map_zip_err)?,
                EntryAction::Rename(new_name) => {
                    writer.raw_copy_file_rename(file, new_name).map_err(map_zip_err)?
                }
            }
        }
        writer.finish().map_err(map_zip_err)?;
    }

    fs::rename(&tmp_path, archive).map_err(|e| Error::io(archive, e))?;
    Ok(())
}

enum EntryAction {
    Keep,
    Skip,
    Rename(String),
}

/// FR-18/19 — xoá các entry có tên khớp `names` khỏi file `.zip`.
pub fn remove_entries(archive: &Path, names: &[String]) -> Result<()> {
    rewrite_archive(archive, |name| {
        if names.iter().any(|n| n == name) {
            EntryAction::Skip
        } else {
            EntryAction::Keep
        }
    })
}

/// FR-18/19 — đổi tên 1 entry trong file `.zip`.
pub fn rename_entry(archive: &Path, old_name: &str, new_name: &str) -> Result<()> {
    let mut found = false;
    rewrite_archive(archive, |name| {
        if name == old_name {
            found = true;
            EntryAction::Rename(new_name.to_string())
        } else {
            EntryAction::Keep
        }
    })?;
    if !found {
        return Err(Error::Archive(format!("không tìm thấy entry: {old_name}")));
    }
    Ok(())
}

pub fn test_integrity(archive_path: &Path, password: Option<&str>) -> Result<bool> {
    let mut archive = open_archive(archive_path)?;
    for i in 0..archive.len() {
        let mut entry = match password {
            Some(pw) => archive.by_index_decrypt(i, pw.as_bytes()),
            None => archive.by_index(i),
        }
        .map_err(map_zip_err)?;
        if entry.is_dir() {
            continue;
        }
        io::copy(&mut entry, &mut io::sink()).map_err(|e| Error::io(archive_path, e))?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compress, extract, list_entries, test_integrity};
    use std::fs;

    fn make_sample_dir(root: &Path) {
        fs::create_dir_all(root.join("thư mục con")).unwrap();
        fs::write(root.join("hello.txt"), b"xin chao viet nam").unwrap();
        fs::write(root.join("thư mục con/tệp có dấu.txt"), "nội dung tiếng Việt".as_bytes()).unwrap();
    }

    #[test]
    fn roundtrip_without_password() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        make_sample_dir(&src_dir);

        let archive = tmp.path().join("out.zip");
        let options = CompressOptions::default();
        compress(&[src_dir.clone()], &archive, &options).unwrap();

        assert!(test_integrity(&archive, None).unwrap());

        let entries = list_entries(&archive, None).unwrap();
        assert!(entries.iter().any(|e| e.name.ends_with("hello.txt")));
        assert!(entries
            .iter()
            .any(|e| e.name.contains("tệp có dấu.txt")));

        let dest_dir = tmp.path().join("out");
        extract(&archive, &dest_dir, None).unwrap();
        let extracted = fs::read_to_string(dest_dir.join("src/hello.txt")).unwrap();
        assert_eq!(extracted, "xin chao viet nam");
        let extracted_vn =
            fs::read_to_string(dest_dir.join("src/thư mục con/tệp có dấu.txt")).unwrap();
        assert_eq!(extracted_vn, "nội dung tiếng Việt");
    }

    #[test]
    fn add_entries_appends_without_touching_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        make_sample_dir(&src_dir);
        let archive = tmp.path().join("out.zip");
        compress(&[src_dir], &archive, &CompressOptions::default()).unwrap();

        let extra = tmp.path().join("extra.txt");
        fs::write(&extra, b"file them vao sau").unwrap();
        add_entries(&archive, &[extra], None, CompressionLevel::Normal).unwrap();

        assert!(test_integrity(&archive, None).unwrap());
        let entries = list_entries(&archive, None).unwrap();
        assert!(entries.iter().any(|e| e.name.ends_with("hello.txt")), "entry cũ phải còn nguyên");
        assert!(entries.iter().any(|e| e.name == "extra.txt"), "entry mới phải được thêm vào");
    }

    #[test]
    fn remove_entries_deletes_only_named_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        make_sample_dir(&src_dir);
        let archive = tmp.path().join("out.zip");
        compress(&[src_dir], &archive, &CompressOptions::default()).unwrap();

        let before = list_entries(&archive, None).unwrap();
        let target = before
            .iter()
            .find(|e| e.name.ends_with("hello.txt"))
            .unwrap()
            .name
            .clone();

        remove_entries(&archive, &[target.clone()]).unwrap();

        assert!(test_integrity(&archive, None).unwrap());
        let after = list_entries(&archive, None).unwrap();
        assert!(!after.iter().any(|e| e.name == target), "entry đã xoá không được còn");
        assert_eq!(after.len(), before.len() - 1, "chỉ đúng 1 entry bị xoá, các entry khác giữ nguyên");
    }

    #[test]
    fn remove_entries_preserves_password_without_needing_it() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        make_sample_dir(&src_dir);
        let archive = tmp.path().join("secret.zip");
        let options = CompressOptions {
            password: Some("mat-khau-vi-du".to_string()),
            ..Default::default()
        };
        compress(&[src_dir], &archive, &options).unwrap();

        let before = list_entries(&archive, Some("mat-khau-vi-du")).unwrap();
        let target = before.iter().find(|e| e.name.ends_with("hello.txt")).unwrap().name.clone();

        // Xoá entry mà KHÔNG cần mật khẩu — raw_copy_file không giải mã, chỉ copy byte nén.
        remove_entries(&archive, &[target.clone()]).unwrap();

        // Các entry còn lại vẫn phải cần đúng mật khẩu cũ để đọc được.
        let err = extract(&archive, &tmp.path().join("wrong"), None).unwrap_err();
        assert!(matches!(err, Error::PasswordRequired));
        let dest = tmp.path().join("ok");
        extract(&archive, &dest, Some("mat-khau-vi-du")).unwrap();
        assert!(!dest.join("src/hello.txt").exists());
        assert!(dest.join("src/thư mục con/tệp có dấu.txt").exists());
    }

    #[test]
    fn rename_entry_changes_name_keeps_content() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        make_sample_dir(&src_dir);
        let archive = tmp.path().join("out.zip");
        compress(&[src_dir], &archive, &CompressOptions::default()).unwrap();

        let before = list_entries(&archive, None).unwrap();
        let old_name = before.iter().find(|e| e.name.ends_with("hello.txt")).unwrap().name.clone();
        let new_name = old_name.replace("hello.txt", "renamed.txt");

        rename_entry(&archive, &old_name, &new_name).unwrap();

        assert!(test_integrity(&archive, None).unwrap());
        let after = list_entries(&archive, None).unwrap();
        assert!(!after.iter().any(|e| e.name == old_name));
        assert!(after.iter().any(|e| e.name == new_name));

        let dest = tmp.path().join("out");
        extract(&archive, &dest, None).unwrap();
        let content = fs::read_to_string(dest.join(&new_name)).unwrap();
        assert_eq!(content, "xin chao viet nam");
    }

    #[test]
    fn rename_entry_errors_when_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        make_sample_dir(&src_dir);
        let archive = tmp.path().join("out.zip");
        compress(&[src_dir], &archive, &CompressOptions::default()).unwrap();

        let err = rename_entry(&archive, "khong-ton-tai.txt", "moi.txt").unwrap_err();
        assert!(matches!(err, Error::Archive(_)));
    }

    /// Đa luồng: nén ≥2 nguồn top-level phải đi qua `compress_parallel` (ngưỡng
    /// `PARALLEL_MIN_UNITS`) và archive kết quả phải đúng như nén tuần tự — mọi entry
    /// từ mọi nguồn đều có mặt, đủ, và giải nén ra đúng nội dung byte-for-byte.
    #[test]
    fn parallel_compress_multiple_sources_matches_sequential_content() {
        let tmp = tempfile::tempdir().unwrap();
        let mut sources = Vec::new();
        for i in 0..5 {
            let dir = tmp.path().join(format!("src{i}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("data.txt"), format!("noi dung nguon so {i}")).unwrap();
            sources.push(dir);
        }

        let archive = tmp.path().join("parallel.zip");
        compress(&sources, &archive, &CompressOptions::default()).unwrap();

        assert!(test_integrity(&archive, None).unwrap());
        let entries = list_entries(&archive, None).unwrap();
        for i in 0..5 {
            assert!(
                entries.iter().any(|e| e.name.ends_with(&format!("src{i}/data.txt"))),
                "thieu entry cua nguon src{i}"
            );
        }

        let dest = tmp.path().join("out");
        extract(&archive, &dest, None).unwrap();
        for i in 0..5 {
            let content = fs::read_to_string(dest.join(format!("src{i}/data.txt"))).unwrap();
            assert_eq!(content, format!("noi dung nguon so {i}"));
        }
    }

    /// Đa luồng + mật khẩu: entry AES-256 được nén ở luồng riêng rồi `raw_copy_file` (copy
    /// nguyên byte, không giải mã) sang archive cuối — phải vẫn giải mã đúng bằng mật khẩu
    /// gốc, xác nhận việc ghép archive không làm hỏng dữ liệu đã mã hoá.
    #[test]
    fn parallel_compress_preserves_password_across_merged_parts() {
        let tmp = tempfile::tempdir().unwrap();
        let mut sources = Vec::new();
        for i in 0..3 {
            let file = tmp.path().join(format!("file{i}.txt"));
            fs::write(&file, format!("bi mat so {i}")).unwrap();
            sources.push(file);
        }

        let archive = tmp.path().join("secret_parallel.zip");
        let options = CompressOptions {
            password: Some("mat-khau-song-song".to_string()),
            ..Default::default()
        };
        compress(&sources, &archive, &options).unwrap();

        let err = extract(&archive, &tmp.path().join("wrong"), None).unwrap_err();
        assert!(matches!(err, Error::PasswordRequired));

        let dest = tmp.path().join("ok");
        extract(&archive, &dest, Some("mat-khau-song-song")).unwrap();
        for i in 0..3 {
            let content = fs::read_to_string(dest.join(format!("file{i}.txt"))).unwrap();
            assert_eq!(content, format!("bi mat so {i}"));
        }
    }

    /// Nén 1 thư mục DUY NHẤT (không phải nhiều nguồn) với nhiều file con phải đi qua
    /// `compress_single_folder`/`compress_parallel_units` — trước đây case này luôn tuần
    /// tự bên trong thư mục dù có nhiều file con. Xác nhận archive kết quả đầy đủ đúng như
    /// nén tuần tự: mọi entry (kể cả entry thư mục gốc, ghi qua `pre_directories`) đều có
    /// mặt và giải nén ra đúng nội dung byte-for-byte.
    #[test]
    fn parallel_compress_single_folder_with_many_children_matches_sequential_content() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("bo-anh");
        fs::create_dir_all(&src_dir).unwrap();
        for i in 0..8 {
            fs::write(src_dir.join(format!("anh{i}.txt")), format!("noi dung anh so {i}")).unwrap();
        }
        fs::create_dir_all(src_dir.join("thu-muc-con")).unwrap();
        fs::write(src_dir.join("thu-muc-con/ghi-chu.txt"), "ghi chu tieng Viet").unwrap();

        let archive = tmp.path().join("bo-anh.zip");
        compress(&[src_dir.clone()], &archive, &CompressOptions::default()).unwrap();

        assert!(test_integrity(&archive, None).unwrap());
        let entries = list_entries(&archive, None).unwrap();
        for i in 0..8 {
            assert!(
                entries.iter().any(|e| e.name.ends_with(&format!("anh{i}.txt"))),
                "thieu entry anh{i}.txt"
            );
        }
        assert!(entries.iter().any(|e| e.name.ends_with("ghi-chu.txt")));

        let dest = tmp.path().join("out");
        extract(&archive, &dest, None).unwrap();
        for i in 0..8 {
            let content = fs::read_to_string(dest.join("bo-anh").join(format!("anh{i}.txt"))).unwrap();
            assert_eq!(content, format!("noi dung anh so {i}"));
        }
        let note = fs::read_to_string(dest.join("bo-anh/thu-muc-con/ghi-chu.txt")).unwrap();
        assert_eq!(note, "ghi chu tieng Viet");
    }

    /// Đường song song cho 1 thư mục + mật khẩu: các entry AES-256 được nén ở luồng riêng
    /// rồi ghép bằng `raw_copy_file` phải vẫn giải mã đúng bằng mật khẩu gốc.
    #[test]
    fn parallel_compress_single_folder_preserves_password() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("bi-mat");
        fs::create_dir_all(&src_dir).unwrap();
        for i in 0..4 {
            fs::write(src_dir.join(format!("file{i}.txt")), format!("bi mat so {i}")).unwrap();
        }

        let archive = tmp.path().join("bi-mat.zip");
        let options = CompressOptions {
            password: Some("mat-khau-thu-muc".to_string()),
            ..Default::default()
        };
        compress(&[src_dir], &archive, &options).unwrap();

        let err = extract(&archive, &tmp.path().join("wrong"), None).unwrap_err();
        assert!(matches!(err, Error::PasswordRequired));

        let dest = tmp.path().join("ok");
        extract(&archive, &dest, Some("mat-khau-thu-muc")).unwrap();
        for i in 0..4 {
            let content = fs::read_to_string(dest.join("bi-mat").join(format!("file{i}.txt"))).unwrap();
            assert_eq!(content, format!("bi mat so {i}"));
        }
    }

    /// Thư mục với dưới `PARALLEL_MIN_UNITS` con (ở đây: 1) phải rơi về đường tuần tự,
    /// không cố chia luồng cho 1 đơn vị duy nhất — vẫn phải nén đúng.
    #[test]
    fn single_folder_with_one_child_falls_back_to_sequential() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("mot-file");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("only.txt"), "noi dung duy nhat").unwrap();

        let archive = tmp.path().join("mot-file.zip");
        compress(&[src_dir], &archive, &CompressOptions::default()).unwrap();

        assert!(test_integrity(&archive, None).unwrap());
        let dest = tmp.path().join("out");
        extract(&archive, &dest, None).unwrap();
        let content = fs::read_to_string(dest.join("mot-file/only.txt")).unwrap();
        assert_eq!(content, "noi dung duy nhat");
    }

    #[test]
    fn roundtrip_with_aes256_password() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        make_sample_dir(&src_dir);

        let archive = tmp.path().join("secret.zip");
        let options = CompressOptions {
            password: Some("mat-khau-vi-du".to_string()),
            ..Default::default()
        };
        compress(&[src_dir], &archive, &options).unwrap();

        // Không có mật khẩu -> phải báo cần mật khẩu, không được đọc lén được.
        let err = extract(&archive, &tmp.path().join("wrong"), None).unwrap_err();
        assert!(matches!(err, Error::PasswordRequired));

        // Sai mật khẩu -> báo sai mật khẩu.
        let err = extract(&archive, &tmp.path().join("wrong2"), Some("sai-roi")).unwrap_err();
        assert!(matches!(err, Error::WrongPassword));

        // Đúng mật khẩu -> giải nén thành công, nội dung đúng.
        let dest_dir = tmp.path().join("ok");
        extract(&archive, &dest_dir, Some("mat-khau-vi-du")).unwrap();
        let extracted = fs::read_to_string(dest_dir.join("src/hello.txt")).unwrap();
        assert_eq!(extracted, "xin chao viet nam");
    }
}
