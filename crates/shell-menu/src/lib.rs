//! COM shell extension DLL cho FR-22: menu chuột phải kiểu 7-Zip (1 mục cha "Vietzip" xổ ra
//! nhiều mục con — Xem/Giải nén/Kiểm tra/Thêm vào archive), với nhãn ĐỘNG (vd
//! `Giải nén vào "Photos"`, dùng đúng tên archive đang bấm chuột phải) — điều mà verb tĩnh
//! (Compress to .zip / Extract Here / Extract to subfolder, vẫn giữ nguyên bên dưới làm
//! phương án dự phòng) không làm được vì Windows đọc nhãn thẳng từ registry, không tính toán
//! được theo file cụ thể. Xem `apps/desktop/src-tauri/windows/shell-context-menu.wxs` để biết
//! phần đăng ký, và CLAUDE.md mục "Dynamic per-file shell-menu labels" cho bối cảnh đầy đủ,
//! bao gồm mức độ rủi ro đã được xác nhận rõ với người dùng trước khi làm mục này.
//!
//! **Verification gap ghi rõ, không giấu**: không có Explorer thật để test tương tác trong
//! môi trường này (không có máy tính bảng điều khiển GUI). Đã build bằng `windows-rs` (binding
//! chính thức của Microsoft, không phải tự viết FFI tay), xác nhận từng API (IContextMenu_Impl,
//! IShellExtInit_Impl, FORMATETC/STGMEDIUM, DragQueryFileW, InsertMenuW/MF_POPUP, công thức
//! HRESULT trả về của QueryContextMenu...) bằng cách đọc thẳng mã nguồn đã vendor của
//! `windows`/`windows-core` VÀ tài liệu chính thức của Microsoft cho từng hàm Win32 — không
//! đoán từ tóm tắt. Việc đăng ký COM thật + Explorer thật gọi đúng
//! `QueryContextMenu`/`InvokeCommand` chỉ có thể xác nhận bằng cách cài MSI thật rồi bấm chuột
//! phải thật, việc mà môi trường này không làm được.
//! **Trước khi coi tính năng này là đáng tin cậy, cần 1 người có máy Windows thật cài thử.**

#[cfg(windows)]
mod class_factory;
#[cfg(windows)]
mod context_menu;
#[cfg(windows)]
mod hdrop;

#[cfg(windows)]
pub mod imp {
    use crate::class_factory::VietzipClassFactory;
    use std::ffi::c_void;
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;
    use windows::core::{BOOL, ComObject, IUnknownImpl, PCWSTR, GUID, HRESULT};
    use windows::Win32::Foundation::{CLASS_E_CLASSNOTAVAILABLE, E_INVALIDARG, HINSTANCE, HMODULE, S_OK};
    use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
    use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK};

    /// CLSID cố định của handler — PHẢI khớp đúng GUID đăng ký trong
    /// `apps/desktop/src-tauri/windows/shell-context-menu.wxs`. Sinh 1 lần bằng thật
    /// `[guid]::NewGuid()` (PowerShell, không phải tự bịa) — không được đổi sau khi phát hành.
    pub const CLSID_VIETZIP_CONTEXT_MENU: GUID = GUID::from_u128(0xe8c419d0_b877_4f44_a4d0_8a1d17d81e20);

    /// Không cho Windows mở kèm 1 cửa sổ console đen khi tự spawn tiến trình con từ trong DLL
    /// (khác lúc Explorer tự gọi thẳng lệnh trong registry cho các verb tĩnh — đó là hành vi
    /// của Explorer, không thuộc phạm vi kiểm soát của DLL này). `vietzip.exe`/`vietzip-desktop.exe`
    /// đều không cần 1 console mới ở đây.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// Handle module của chính DLL này — set 1 lần ở `DllMain`, dùng để tự định vị đường dẫn
    /// của mình rồi suy ra các binary khác nằm cạnh (cùng `INSTALLDIR`, giống cách `sfx-stub`
    /// và CLI đã tự tìm nhau qua `current_exe().parent()`).
    static DLL_MODULE: OnceLock<usize> = OnceLock::new();

    #[unsafe(no_mangle)]
    extern "system" fn DllMain(module: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
        if reason == DLL_PROCESS_ATTACH {
            let _ = DLL_MODULE.set(module.0 as usize);
        }
        BOOL(1)
    }

    /// Tìm 1 file tên `name` cạnh chính DLL này (`INSTALLDIR` sau khi cài qua MSI). Trả lỗi rõ
    /// ràng thay vì panic nếu không tìm thấy — mọi hàm gọi tới đây đều nằm sau `catch_unwind`
    /// ở `context_menu.rs`, nhưng lỗi rõ ràng vẫn tốt hơn để dễ chẩn đoán nếu cần.
    fn find_sibling(name: &str) -> std::io::Result<PathBuf> {
        let handle = DLL_MODULE
            .get()
            .copied()
            .ok_or_else(|| std::io::Error::other("chưa xác định được đường dẫn của chính DLL này"))?;
        let hmodule = HMODULE(handle as *mut c_void);

        let mut buffer = vec![0u16; 32 * 1024];
        let len = unsafe { GetModuleFileNameW(Some(hmodule), &mut buffer) };
        if len == 0 {
            return Err(std::io::Error::last_os_error());
        }
        buffer.truncate(len as usize);
        let dll_path = PathBuf::from(String::from_utf16_lossy(&buffer));

        let dir = dll_path
            .parent()
            .ok_or_else(|| std::io::Error::other("đường dẫn DLL không có thư mục cha"))?;
        let sibling = dir.join(name);
        if !sibling.exists() {
            return Err(std::io::Error::other(format!("không tìm thấy {}", sibling.display())));
        }
        Ok(sibling)
    }

    fn show_message_box(message: &str, title: &str, is_error: bool) {
        let msg_wide: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
        let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let icon = if is_error { MB_ICONERROR } else { MB_ICONINFORMATION };
        unsafe {
            MessageBoxW(None, PCWSTR(msg_wide.as_ptr()), PCWSTR(title_wide.as_ptr()), MB_OK | icon);
        }
    }

    /// "Giải nén tại đây" / "Giải nén vào thư mục con" — dùng lại đúng logic `--here`/
    /// `--to-subfolder` mà verb tĩnh (FR-22) đã dùng, xem `crates/cli/src/main.rs::extract_dest`.
    /// Không chờ tiến trình con (`spawn`, không `wait`) — handler chỉ có nhiệm vụ khởi chạy,
    /// giữ đúng nguyên tắc "không lặp lại logic archive trong DLL" đã nêu ở đầu file.
    pub(crate) fn spawn_extract(archive: &Path, here: bool) -> std::io::Result<()> {
        let cli = find_sibling("vietzip.exe")?;
        std::process::Command::new(cli)
            .arg("extract")
            .arg(archive)
            .arg(if here { "--here" } else { "--to-subfolder" })
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
        Ok(())
    }

    /// "Xem nội dung" / "Thêm vào archive" — thay vì tự dựng UI xem/thêm-file NGAY TRONG DLL
    /// COM (rủi ro cao hơn nhiều — mọi lỗi UI ở đây đều chạy trong `explorer.exe`), mở lại
    /// chính app Desktop kèm 1 cờ dòng lệnh; app tự lo phần UI bằng luồng đã có sẵn — xem
    /// `apps/desktop/src-tauri/src/lib.rs::get_launch_intent` +
    /// `apps/desktop/src/main.ts::applyLaunchIntent`. `vietzip-desktop.exe` là ứng dụng GUI
    /// (đã `windows_subsystem = "windows"` ở bản release) nên không cần `CREATE_NO_WINDOW`.
    pub(crate) fn spawn_desktop_app(flag: &str, path: &Path) -> std::io::Result<()> {
        let app = find_sibling("vietzip-desktop.exe")?;
        std::process::Command::new(app).arg(flag).arg(path).spawn()?;
        Ok(())
    }

    /// "Kiểm tra file nén" — khác các mục còn lại, cần CHỜ kết quả để báo pass/fail, nhưng
    /// `InvokeCommand` (nơi gọi hàm này) không nên tự chặn luồng UI của Explorer trong lúc
    /// chờ — chạy `wait` + hộp thoại kết quả trên 1 thread nền riêng, `InvokeCommand` trả về
    /// ngay lập tức. Dựa vào exit code thật của CLI (0 = hợp lệ, khác 0 = có vấn đề — sửa lại
    /// đúng chỗ này ở `crates/cli/src/main.rs::Command::Test`, trước đây bị bỏ qua âm thầm).
    pub(crate) fn spawn_test_with_result_dialog(archive: &Path) -> std::io::Result<()> {
        let cli = find_sibling("vietzip.exe")?;
        let archive = archive.to_path_buf();
        std::thread::spawn(move || {
            let output = std::process::Command::new(&cli)
                .arg("test")
                .arg(&archive)
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            let name = archive.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            match output {
                Ok(out) if out.status.success() => {
                    show_message_box(&format!("\"{name}\" hợp lệ, không có lỗi."), "Vietzip — Kiểm tra", false);
                }
                Ok(out) => {
                    let detail = String::from_utf8_lossy(&out.stderr);
                    show_message_box(&format!("\"{name}\" có vấn đề:\n{detail}"), "Vietzip — Kiểm tra", true);
                }
                Err(e) => {
                    show_message_box(&format!("Không chạy được vietzip.exe: {e}"), "Vietzip — Kiểm tra", true);
                }
            }
        });
        Ok(())
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn DllGetClassObject(rclsid: *const GUID, riid: *const GUID, ppv: *mut *mut c_void) -> HRESULT {
        if ppv.is_null() {
            return E_INVALIDARG;
        }
        unsafe { *ppv = std::ptr::null_mut() };
        if rclsid.is_null() || riid.is_null() {
            return E_INVALIDARG;
        }
        if unsafe { *rclsid } != CLSID_VIETZIP_CONTEXT_MENU {
            return CLASS_E_CLASSNOTAVAILABLE;
        }

        let factory: ComObject<VietzipClassFactory> = ComObject::new(VietzipClassFactory);
        // Xem giải thích refcount tương tự ở `class_factory.rs::CreateInstance` — `factory`
        // drop ngay sau khi trả về chỉ đưa refcount về đúng 1 tham chiếu caller sở hữu.
        match unsafe { factory.QueryInterface(riid, ppv).ok() } {
            Ok(()) => S_OK,
            Err(e) => e.code(),
        }
    }

    /// Đơn giản hoá có chủ đích: luôn báo "có thể unload" thay vì tự đếm số object COM còn
    /// sống. Mỗi `VietzipContextMenu` chỉ sống trong đúng 1 lượt bấm chuột phải (Explorer tự
    /// giải phóng khi đóng menu) — không có trạng thái nào cần giữ DLL sống lâu hơn thực tế
    /// cần, nên bỏ qua việc đếm chính xác không tạo rủi ro thật (khác các phần khác của DLL
    /// này, đây không phải chỗ cần cẩn trọng thêm).
    #[unsafe(no_mangle)]
    extern "system" fn DllCanUnloadNow() -> HRESULT {
        S_OK
    }
}
