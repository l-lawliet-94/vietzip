//! CLI cơ bản — dùng để kiểm chứng core engine trước khi đầu tư vào UI (Giai đoạn 3, ke-hoach-mvp.md).

use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "vietzip",
    version,
    about = "Công cụ nén/giải nén dòng lệnh (CLI)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// FR-03. Ánh xạ sang `vietzip_core::CompressionLevel`.
#[derive(Clone, Copy, ValueEnum)]
enum Level {
    Fast,
    Normal,
    Ultra,
}

impl From<Level> for vietzip_core::CompressionLevel {
    fn from(level: Level) -> Self {
        match level {
            Level::Fast => vietzip_core::CompressionLevel::Fast,
            Level::Normal => vietzip_core::CompressionLevel::Normal,
            Level::Ultra => vietzip_core::CompressionLevel::Ultra,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Nén file/thư mục thành 1 file lưu trữ (.zip hoặc .7z)
    Compress {
        /// File hoặc thư mục nguồn (có thể liệt kê nhiều)
        #[arg(required = true)]
        sources: Vec<PathBuf>,
        /// File nén đầu ra — đuôi mở rộng quyết định định dạng (.zip, .7z). Nếu bỏ trống,
        /// tự đặt tên `<tên-nguồn-đầu-tiên>.zip` cạnh nguồn đó — dùng cho context-menu
        /// "Compress to [name].zip" (FR-22), nơi Explorer chỉ truyền đường dẫn đã chọn,
        /// không có chỗ để người dùng gõ thêm tham số.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Đặt mật khẩu, mã hóa AES-256 (ZIP và 7Z đều hỗ trợ)
        #[arg(short, long)]
        password: Option<String>,
        /// Mức độ nén — FR-03 (mặc định: normal)
        #[arg(short, long, value_enum, default_value = "normal")]
        level: Level,
    },
    /// Giải nén 1 file lưu trữ
    Extract {
        archive: PathBuf,
        /// Thư mục đích (mặc định: thư mục hiện tại). Bỏ qua nếu dùng `--here`/`--to-subfolder`.
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        /// FR-22: giải nén thẳng vào cùng thư mục chứa file nén ("Extract Here")
        #[arg(long, conflicts_with = "output")]
        here: bool,
        /// FR-22: giải nén vào thư mục con mới cùng tên với file nén ("Extract to <name>\")
        #[arg(long, conflicts_with_all = ["output", "here"])]
        to_subfolder: bool,
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Liệt kê nội dung bên trong file nén, không giải nén
    List {
        archive: PathBuf,
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Kiểm tra tính toàn vẹn của file nén (Test Archive)
    Test {
        archive: PathBuf,
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Chia 1 file thành nhiều phần (FR-04) — thường dùng sau khi nén, để gửi qua
    /// email/USB. KHÔNG phải multi-volume ZIP/7z gốc — dùng lệnh `join` để ghép lại
    /// trước khi giải nén, không mở trực tiếp phần đầu tiên bằng công cụ khác.
    Split {
        file: PathBuf,
        /// Kích thước mỗi phần, tính bằng MB
        #[arg(short, long)]
        size_mb: u64,
    },
    /// Ghép các phần đã chia bằng `split` lại thành 1 file
    Join {
        /// Phần đầu tiên (đuôi .001)
        first_part: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Tạo file tự giải nén (.exe) từ 1 file .zip/.7z đã nén sẵn (FR-07)
    Sfx {
        /// File nén nguồn (.zip hoặc .7z)
        archive: PathBuf,
        /// File .exe đầu ra
        #[arg(short, long)]
        output: PathBuf,
        /// Đường dẫn tới stub đã biên dịch sẵn (sfx-stub.exe) — mặc định: cùng thư mục
        /// với chính file `vietzip` (CLI) đang chạy, khớp cách cả 2 được build cùng
        /// `target/` trong workspace này.
        #[arg(long)]
        stub: Option<PathBuf>,
        /// Đường dẫn (tương đối so với thư mục vừa giải nén) tới 1 chương trình để tự chạy
        /// ngay sau khi giải nén xong, ví dụ "setup.exe" — tương đương rút gọn của
        /// `RunProgram` trong SFX config.txt của 7-Zip. Bỏ trống nếu chỉ muốn giải nén thụ động.
        #[arg(long)]
        run: Option<String>,
    },
    /// Thêm file/thư mục vào 1 file .zip đã có sẵn (FR-18/19), không nén lại từ đầu
    Add {
        archive: PathBuf,
        #[arg(required = true)]
        sources: Vec<PathBuf>,
        /// Mật khẩu cho các entry MỚI thêm vào (không đổi mật khẩu của entry đã có sẵn)
        #[arg(short, long)]
        password: Option<String>,
        #[arg(short, long, value_enum, default_value = "normal")]
        level: Level,
    },
    /// Xoá 1 hoặc nhiều entry khỏi file .zip theo tên (FR-18/19)
    Remove {
        archive: PathBuf,
        #[arg(required = true)]
        names: Vec<String>,
    },
    /// Đổi tên 1 entry trong file .zip (FR-18/19)
    Rename {
        archive: PathBuf,
        old_name: String,
        new_name: String,
    },
    /// Đo tốc độ nén/giải nén trên máy này (tương đương Tools > Benchmark của 7-Zip)
    Benchmark {
        /// Kích thước dữ liệu tổng hợp dùng để đo, tính bằng MB
        #[arg(short, long, default_value_t = 64)]
        size_mb: u64,
        #[arg(short, long, value_enum, default_value = "normal")]
        level: Level,
    },
    /// Tính CRC32 và SHA-256 của 1 file bất kỳ (không liên quan tới archive)
    Checksum { file: PathBuf },
    /// Chuyển đổi 1 file nén sang định dạng khác (giải nén rồi nén lại — 7-Zip cũng không
    /// có "chuyển đổi trực tiếp" thật, đây gộp 2 bước thủ công thành 1 lệnh)
    Convert {
        source: PathBuf,
        /// File đích — đuôi mở rộng quyết định định dạng mới
        #[arg(short, long)]
        output: PathBuf,
        /// Mật khẩu của file NGUỒN (để giải nén), nếu có
        #[arg(long)]
        source_password: Option<String>,
        /// Mật khẩu cho file ĐÍCH (để nén lại), nếu muốn đặt/đổi mật khẩu
        #[arg(long)]
        dest_password: Option<String>,
        #[arg(short, long, value_enum, default_value = "normal")]
        level: Level,
    },
    /// Sửa file nén bị lỗi (FR-17) — mức hỗ trợ tuỳ định dạng: ZIP phục hồi dữ liệu thật
    /// (quét lại từng entry theo local header khi central directory hỏng), .7z chỉ phát
    /// hiện lỗi (không có kỹ thuật phục hồi an toàn), định dạng khác không hỗ trợ.
    Repair {
        archive: PathBuf,
        /// File đích chứa kết quả sửa
        #[arg(short, long)]
        output: PathBuf,
        /// Mật khẩu, dùng để thử mở archive theo đường bình thường trước khi quét thô
        #[arg(short, long)]
        password: Option<String>,
    },
}

/// Tìm `sfx-stub(.exe)` mặc định: cùng thư mục với binary CLI đang chạy (cả hai đều nằm
/// trong `target/<profile>/` sau `cargo build --workspace`).
fn default_stub_path() -> Result<PathBuf, String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = current_exe
        .parent()
        .ok_or_else(|| "Không xác định được thư mục chứa CLI".to_string())?;
    let stub_name = if cfg!(windows) { "sfx-stub.exe" } else { "sfx-stub" };
    let stub = dir.join(stub_name);
    if !stub.exists() {
        return Err(format!(
            "Không tìm thấy {} — hãy build cùng workspace (`cargo build --workspace`) hoặc chỉ định --stub",
            stub.display()
        ));
    }
    Ok(stub)
}

/// FR-22: "Compress to [name].zip" — Explorer chỉ truyền các đường dẫn đã chọn, không có
/// chỗ để nhập tên file đích, nên tự đặt tên theo nguồn đầu tiên (khớp cách 7-Zip đặt tên
/// mặc định), luôn ra `.zip` (định dạng mặc định, không cho chọn `.7z` từ context-menu).
fn derive_compress_output(sources: &[PathBuf]) -> Result<PathBuf, String> {
    let first = sources.first().ok_or("cần ít nhất 1 nguồn để nén")?;
    let parent = first.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let name = if first.is_dir() { first.file_name() } else { first.file_stem() }
        .ok_or_else(|| format!("không xác định được tên file nén đầu ra cho: {}", first.display()))?;
    let mut out = parent.join(name);
    out.set_extension("zip");
    Ok(out)
}

/// FR-22: "Extract Here" (`--here`) / "Extract to <name>\" (`--to-subfolder`) đều suy ra
/// thư mục đích từ chính đường dẫn archive, không cần người dùng gõ thêm gì trên context-menu.
fn extract_dest(archive: &Path, output: PathBuf, here: bool, to_subfolder: bool) -> PathBuf {
    let parent = archive.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    if here {
        parent.to_path_buf()
    } else if to_subfolder {
        parent.join(archive.file_stem().unwrap_or_default())
    } else {
        output
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Compress {
            sources,
            output,
            password,
            level,
        } => {
            let output = match output.map(Ok).unwrap_or_else(|| derive_compress_output(&sources)) {
                Ok(path) => path,
                Err(msg) => {
                    eprintln!("Lỗi: {msg}");
                    return ExitCode::FAILURE;
                }
            };
            let options = vietzip_core::CompressOptions {
                password,
                level: level.into(),
            };
            vietzip_core::compress(&sources, &output, &options)
        }
        Command::Extract {
            archive,
            output,
            here,
            to_subfolder,
            password,
        } => {
            let dest = extract_dest(&archive, output, here, to_subfolder);
            vietzip_core::extract(&archive, &dest, password.as_deref())
        }
        Command::List { archive, password } => {
            vietzip_core::list_entries(&archive, password.as_deref()).map(|entries| {
                for entry in entries {
                    let marker = if entry.is_dir { "d" } else { "-" };
                    println!("{marker} {:>12} {}", entry.size, entry.name);
                }
            })
        }
        Command::Test { archive, password } => {
            vietzip_core::test_integrity(&archive, password.as_deref()).and_then(|ok| {
                if ok {
                    println!("OK: file nén hợp lệ.");
                    Ok(())
                } else {
                    // Trước đây bị bỏ qua âm thầm (kết quả `false` không lỗi vẫn thoát mã 0)
                    // — sửa để `test` phản ánh đúng qua exit code, cần thiết cho mục "Kiểm
                    // tra" mới trong menu chuột phải (COM shell extension) dựa vào exit code
                    // để biết hiển thị thông báo pass/fail nào.
                    Err(vietzip_core::Error::Archive("File nén có vấn đề (kiểm tra thất bại).".to_string()))
                }
            })
        }
        Command::Split { file, size_mb } => {
            vietzip_core::split_file(&file, size_mb * 1024 * 1024).map(|parts| {
                for part in &parts {
                    println!("{}", part.display());
                }
                println!("Đã chia thành {} phần.", parts.len());
            })
        }
        Command::Join { first_part, output } => {
            vietzip_core::join_parts(&first_part, &output).map(|()| {
                println!("Đã ghép vào: {}", output.display());
            })
        }
        Command::Sfx { archive, output, stub, run } => {
            let stub_path = match stub.map(Ok).unwrap_or_else(default_stub_path) {
                Ok(path) => path,
                Err(msg) => {
                    eprintln!("Lỗi: {msg}");
                    return ExitCode::FAILURE;
                }
            };
            vietzip_core::write_sfx(&stub_path, &archive, &output, run.as_deref()).map(|()| {
                println!("Đã tạo file tự giải nén: {}", output.display());
            })
        }
        Command::Add { archive, sources, password, level } => {
            vietzip_core::add_entries(&archive, &sources, password.as_deref(), level.into()).map(|()| {
                println!("Đã thêm {} mục vào: {}", sources.len(), archive.display());
            })
        }
        Command::Remove { archive, names } => vietzip_core::remove_entries(&archive, &names).map(|()| {
            println!("Đã xoá {} mục khỏi: {}", names.len(), archive.display());
        }),
        Command::Rename { archive, old_name, new_name } => {
            vietzip_core::rename_entry(&archive, &old_name, &new_name).map(|()| {
                println!("Đã đổi tên {old_name} -> {new_name} trong: {}", archive.display());
            })
        }
        Command::Benchmark { size_mb, level } => {
            vietzip_core::run_benchmark(size_mb, level.into()).map(|results| {
                println!("Benchmark trên {size_mb} MB dữ liệu tổng hợp:");
                for r in results {
                    println!(
                        "  {:<3}  nén {:>7.1} MB/s   giải nén {:>7.1} MB/s   còn lại {:>5.1}%",
                        r.format,
                        r.compress_mb_per_sec,
                        r.decompress_mb_per_sec,
                        r.compression_ratio_percent()
                    );
                }
            })
        }
        Command::Checksum { file } => vietzip_core::compute_checksum(&file).map(|c| {
            println!("File: {} ({} byte)", file.display(), c.size_bytes);
            println!("CRC32:  {:08x}", c.crc32);
            println!("SHA256: {}", c.sha256_hex);
        }),
        Command::Convert { source, output, source_password, dest_password, level } => {
            let options = vietzip_core::ConvertOptions {
                source_password,
                dest_password,
                level: level.into(),
            };
            vietzip_core::convert(&source, &output, &options).map(|()| {
                println!("Đã chuyển đổi {} -> {}", source.display(), output.display());
            })
        }
        Command::Repair { archive, output, password } => {
            vietzip_core::repair(&archive, &output, password.as_deref()).map(|report| {
                println!("Đã ghi kết quả sửa vào: {}", output.display());
                println!("Phục hồi được {} mục:", report.recovered.len());
                for name in &report.recovered {
                    println!("  + {name}");
                }
                if !report.unrecoverable.is_empty() {
                    println!("Không phục hồi được {} mục:", report.unrecoverable.len());
                    for name in &report.unrecoverable {
                        println!("  - {name}");
                    }
                }
            })
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Lỗi: {err}");
            ExitCode::FAILURE
        }
    }
}

/// FR-22: khoá hành vi của các hàm suy luận đường dẫn dùng cho context-menu Explorer —
/// đây là phần logic duy nhất kiểm thử được ở máy này (phần đăng ký registry/WiX đã xác
/// minh riêng bằng cách đọc trực tiếp bảng Registry trong file .msi đã build, không cài đặt
/// thật — xem CLAUDE.md mục FR-22).
#[cfg(test)]
mod context_menu_tests {
    use super::*;

    #[test]
    fn derive_compress_output_names_after_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let folder = tmp.path().join("MyFolder");
        std::fs::create_dir(&folder).unwrap();

        let out = derive_compress_output(&[folder.clone()]).unwrap();
        assert_eq!(out, tmp.path().join("MyFolder.zip"));
    }

    #[test]
    fn derive_compress_output_names_after_file_stem() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("report.docx");
        std::fs::write(&file, b"x").unwrap();

        let out = derive_compress_output(&[file]).unwrap();
        assert_eq!(out, tmp.path().join("report.zip"));
    }

    #[test]
    fn extract_dest_here_uses_archive_parent() {
        let archive = Path::new("C:/Users/x/Downloads/data.zip");
        let dest = extract_dest(archive, PathBuf::from("."), true, false);
        assert_eq!(dest, Path::new("C:/Users/x/Downloads"));
    }

    #[test]
    fn extract_dest_to_subfolder_uses_archive_stem() {
        let archive = Path::new("C:/Users/x/Downloads/data.zip");
        let dest = extract_dest(archive, PathBuf::from("."), false, true);
        assert_eq!(dest, Path::new("C:/Users/x/Downloads/data"));
    }

    #[test]
    fn extract_dest_falls_back_to_explicit_output() {
        let archive = Path::new("data.zip");
        let dest = extract_dest(archive, PathBuf::from("/custom/dest"), false, false);
        assert_eq!(dest, Path::new("/custom/dest"));
    }
}
