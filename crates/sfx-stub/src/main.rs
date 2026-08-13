//! Stub launcher cho file SFX (FR-07) — được `vietzip_core::write_sfx` nối thêm dữ liệu
//! archive vào cuối để tạo ra 1 file `.exe` tự giải nén. Cố tình chạy dạng console (không
//! GUI, không phụ thuộc WebView2/Tauri runtime) để giữ file SFX nhỏ gọn và không cần cài
//! thêm gì trên máy người nhận — đây là điểm khác biệt có chủ đích so với SFX GUI của
//! 7-Zip/WinRAR, chấp nhận đánh đổi để tránh việc phải đóng gói cả 1 runtime desktop vào
//! từng file SFX (xem CLAUDE.md mục "Scope has grown..." để biết bối cảnh FR-07).
//!
//! Cách dùng: double-click hoặc chạy `installer.exe [--password <mk>] [--dest <thư mục>]`.
//! Mặc định giải nén vào thư mục con cùng tên với file exe, đặt cạnh file exe đó.

use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

fn parse_args() -> (Option<String>, Option<PathBuf>) {
    let mut password = None;
    let mut dest = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--password" => password = args.next(),
            "--dest" => dest = args.next().map(PathBuf::from),
            _ => {}
        }
    }
    (password, dest)
}

fn pause_before_exit() {
    print!("\nNhấn Enter để đóng...");
    let _ = io::stdout().flush();
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
}

fn run() -> Result<(PathBuf, Option<String>), String> {
    let exe_path = env::current_exe().map_err(|e| format!("Không xác định được đường dẫn file exe: {e}"))?;
    let (password, dest_override) = parse_args();

    let dest_dir = dest_override.unwrap_or_else(|| {
        let stem = exe_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "extracted".to_string());
        exe_path.with_file_name(stem)
    });

    let tmp_archive_base = env::temp_dir().join(format!("vietzip-sfx-{}", std::process::id()));
    let info =
        vietzip_core::extract_embedded_archive(&exe_path, &tmp_archive_base).map_err(|e| e.to_string())?;

    let result =
        vietzip_core::extract(&info.archive_path, &dest_dir, password.as_deref()).map_err(|e| e.to_string());
    let _ = std::fs::remove_file(&info.archive_path);
    result?;

    Ok((dest_dir, info.run_after_extract))
}

/// Chạy chương trình được chỉ định trong trailer (nếu có), đường dẫn tương đối so với thư
/// mục vừa giải nén ra. Không chờ chương trình đó chạy xong (`spawn`, không `wait`) — SFX chỉ
/// có nhiệm vụ khởi chạy, không phải giám sát tiến trình con.
fn run_after_extract(dest_dir: &PathBuf, relative_path: &str) {
    let target = dest_dir.join(relative_path);
    println!("Đang chạy: {}", target.display());
    match Command::new(&target).current_dir(dest_dir).spawn() {
        Ok(_) => {}
        Err(err) => eprintln!("Không thể chạy '{}': {err}", target.display()),
    }
}

fn main() {
    println!("Vietzip — file tự giải nén (SFX)");
    match run() {
        Ok((dest_dir, run_cmd)) => {
            println!("Đã giải nén xong vào: {}", dest_dir.display());
            if let Some(relative_path) = run_cmd {
                run_after_extract(&dest_dir, &relative_path);
            }
        }
        Err(err) => eprintln!("Lỗi: {err}"),
    }
    pause_before_exit();
}
