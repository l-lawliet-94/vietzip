//! Đối tượng COM thật — `IContextMenu` + `IShellExtInit`. Xây menu kiểu 7-Zip: 1 mục cha
//! "Vietzip" xổ ra 1 submenu con (Xem nội dung / Giải nén tại đây / Giải nén vào "<Tên>" /
//! Kiểm tra / Thêm vào archive) — thay cho các verb tĩnh phẳng đăng ký qua WiX (vẫn giữ
//! nguyên bên dưới làm phương án dự phòng). `QueryContextMenu` được gọi lại MỖI LẦN người
//! dùng bấm chuột phải nên nhãn "Giải nén vào X" luôn tính lại đúng theo file đang chọn.
//!
//! Cố tình **không** viết lại logic nén/giải nén/xem-nội-dung ở đây — `InvokeCommand` chỉ gọi
//! ra `vietzip.exe`/`vietzip-desktop.exe` (đã kiểm chứng đầy đủ) giống hệt cách verb tĩnh đã
//! làm, giữ toàn bộ bề mặt rủi ro mới chỉ nằm ở phần đăng ký/gọi menu, không phải logic archive.
//!
//! **Mọi phương thức COM đều bọc `catch_unwind`** — 1 panic Rust unwind xuyên qua biên FFI
//! vào `explorer.exe` là hành vi không xác định (có thể crash Explorer của người dùng, không
//! chỉ crash tiến trình của chính app này) — đây là lý do rủi ro thật được nêu khi xin xác
//! nhận trước khi làm mục này, không phải rủi ro lý thuyết.

use crate::hdrop;
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use windows::core::{implement, Result, PCWSTR, PSTR};
use windows::Win32::Foundation::{E_FAIL, E_INVALIDARG, E_NOTIMPL};
use windows::Win32::System::Com::IDataObject;
use windows::Win32::System::Registry::HKEY;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    CMINVOKECOMMANDINFO, CMF_DEFAULTONLY, IContextMenu, IContextMenu_Impl, IShellExtInit, IShellExtInit_Impl,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, InsertMenuW, HMENU, MF_BYPOSITION, MF_POPUP, MF_STRING,
};

/// 5 lệnh trong submenu con, theo đúng thứ tự hiển thị — offset TƯƠNG ĐỐI tính từ `idCmdFirst`
/// mà Explorer cấp ở mỗi lượt gọi `QueryContextMenu` (không phải ID tuyệt đối cố định).
const CMD_VIEW: usize = 0;
const CMD_EXTRACT_HERE: usize = 1;
const CMD_EXTRACT_SUBFOLDER: usize = 2;
const CMD_TEST: usize = 3;
const CMD_ADD_TO_ARCHIVE: usize = 4;
const COMMAND_COUNT: usize = 5;

#[implement(IContextMenu, IShellExtInit)]
pub struct VietzipContextMenu {
    /// File đang được bấm chuột phải — chỉ set khi đúng 1 file, và file đó là 1 định dạng
    /// core nhận diện được (dùng lại `vietzip_core::Format::from_path`, không tự chép 1 danh
    /// sách đuôi mở rộng riêng ở đây — 1 nguồn sự thật duy nhất cho "định dạng nào hỗ trợ").
    selected: RefCell<Option<PathBuf>>,
}

impl Default for VietzipContextMenu {
    fn default() -> Self {
        Self { selected: RefCell::new(None) }
    }
}

impl IShellExtInit_Impl for VietzipContextMenu_Impl {
    fn Initialize(
        &self,
        _pidlfolder: *const ITEMIDLIST,
        pdtobj: windows::core::Ref<'_, IDataObject>,
        _hkeyprogid: HKEY,
    ) -> Result<()> {
        catch_unwind(AssertUnwindSafe(|| {
            let Some(data_object) = pdtobj.as_ref() else {
                return Err(E_INVALIDARG.into());
            };
            let paths = hdrop::extract_dropped_paths(data_object);
            // Chỉ áp dụng khi đúng 1 file được chọn VÀ file đó là định dạng archive nhận diện
            // được — tránh mơ hồ (chọn nhiều file, hoặc 1 file không phải archive thì không
            // thêm gì, thay vì đoán ý người dùng).
            let single = match paths.as_slice() {
                [only] if vietzip_core::Format::from_path(only).is_some() => Some(only.clone()),
                _ => None,
            };
            *self.selected.borrow_mut() = single;
            Ok(())
        }))
        .unwrap_or(Err(E_FAIL.into()))
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

impl IContextMenu_Impl for VietzipContextMenu_Impl {
    fn QueryContextMenu(&self, hmenu: HMENU, indexmenu: u32, idcmdfirst: u32, _idcmdlast: u32, uflags: u32) -> windows::core::HRESULT {
        let result = catch_unwind(AssertUnwindSafe(|| -> windows::core::HRESULT {
            // CMF_DEFAULTONLY: Explorer chỉ đang hỏi verb mặc định (vd double-click) — quy
            // ước chuẩn của Win32 là không thêm gì trong trường hợp này (xem tài liệu
            // IContextMenu::QueryContextMenu của Microsoft). Không thêm gì = trả offset 0.
            if uflags & CMF_DEFAULTONLY != 0 {
                return windows::core::HRESULT(0);
            }

            let Some(path) = self.selected.borrow().clone() else {
                return windows::core::HRESULT(0);
            };
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                return windows::core::HRESULT(0);
            };

            // 5 mục con — nhãn Unicode đầy đủ dấu tiếng Việt (khác verb tĩnh trong WiX, vốn bị
            // giới hạn ASCII vì WiX build bằng codepage 1252; nhãn ở đây do chính DLL dựng lúc
            // chạy, không qua ràng buộc đó).
            let items: [(usize, String); COMMAND_COUNT] = [
                (CMD_VIEW, "Xem nội dung".to_string()),
                (CMD_EXTRACT_HERE, "Giải nén tại đây".to_string()),
                (CMD_EXTRACT_SUBFOLDER, format!("Giải nén vào \"{stem}\"")),
                (CMD_TEST, "Kiểm tra file nén".to_string()),
                (CMD_ADD_TO_ARCHIVE, "Thêm vào archive có sẵn...".to_string()),
            ];

            let Ok(submenu) = (unsafe { CreatePopupMenu() }) else {
                return windows::core::HRESULT(0);
            };

            for (offset, label) in &items {
                let wide = to_wide(label);
                let ok = unsafe {
                    InsertMenuW(
                        submenu,
                        u32::MAX, // MF_BYPOSITION với -1/MAX luôn thêm vào cuối, đúng thứ tự đã liệt kê
                        MF_BYPOSITION | MF_STRING,
                        idcmdfirst as usize + offset,
                        PCWSTR(wide.as_ptr()),
                    )
                };
                if ok.is_err() {
                    unsafe { let _ = DestroyMenu(submenu); }
                    return windows::core::HRESULT(0);
                }
            }

            let top_label = to_wide("Vietzip");
            let inserted = unsafe {
                InsertMenuW(
                    hmenu,
                    indexmenu,
                    MF_BYPOSITION | MF_POPUP,
                    submenu.0 as usize,
                    PCWSTR(top_label.as_ptr()),
                )
            };

            match inserted {
                // Offset TƯƠNG ĐỐI của ID lớn nhất đã dùng (COMMAND_COUNT - 1), cộng 1 — đúng
                // công thức MAKE_HRESULT(SEVERITY_SUCCESS, 0, offset+1) trong tài liệu chính
                // thức của Microsoft cho QueryContextMenu (đã xác nhận qua learn.microsoft.com,
                // không phải suy đoán) — KHÔNG cộng thêm `idcmdfirst` vào giá trị trả về.
                Ok(()) => windows::core::HRESULT(COMMAND_COUNT as i32),
                Err(_) => {
                    unsafe { let _ = DestroyMenu(submenu); }
                    windows::core::HRESULT(0)
                }
            }
        }));
        result.unwrap_or(E_FAIL)
    }

    fn InvokeCommand(&self, pici: *const CMINVOKECOMMANDINFO) -> Result<()> {
        catch_unwind(AssertUnwindSafe(|| {
            if pici.is_null() {
                return Err(E_INVALIDARG.into());
            }
            let info = unsafe { &*pici };

            // `lpVerb` là con trỏ chuỗi CHỈ KHI HIWORD khác 0 — nếu HIWORD = 0, LOWORD chính
            // là ID lệnh (offset tính từ idCmdFirst lúc `QueryContextMenu`), quy ước chuẩn của
            // Win32 (xem tài liệu chính thức của Microsoft cho IContextMenu::InvokeCommand,
            // không phải suy đoán).
            let verb_value = info.lpVerb.0 as usize;
            if verb_value > 0xFFFF {
                // Là con trỏ chuỗi thật, không phải verb theo ID của handler này — bỏ qua.
                return Ok(());
            }

            let Some(path) = self.selected.borrow().clone() else {
                return Err(E_FAIL.into());
            };

            let outcome = match verb_value {
                CMD_VIEW => crate::imp::spawn_desktop_app("--view", &path),
                CMD_EXTRACT_HERE => crate::imp::spawn_extract(&path, true),
                CMD_EXTRACT_SUBFOLDER => crate::imp::spawn_extract(&path, false),
                CMD_TEST => crate::imp::spawn_test_with_result_dialog(&path),
                CMD_ADD_TO_ARCHIVE => crate::imp::spawn_desktop_app("--add-to", &path),
                _ => return Ok(()), // ID lạ (không phải của handler này) — bỏ qua, không lỗi.
            };

            outcome.map_err(|_| E_FAIL.into())
        }))
        .unwrap_or(Err(E_FAIL.into()))
    }

    fn GetCommandString(&self, idcmd: usize, utype: u32, _preserved: *const u32, pszname: PSTR, cchmax: u32) -> Result<()> {
        catch_unwind(AssertUnwindSafe(|| {
            let help_text = match idcmd {
                CMD_VIEW => "Xem nội dung file nén này bằng Vietzip",
                CMD_EXTRACT_HERE => "Giải nén file nén này vào đúng thư mục hiện tại",
                CMD_EXTRACT_SUBFOLDER => "Giải nén file nén này vào 1 thư mục con cùng tên",
                CMD_TEST => "Kiểm tra tính toàn vẹn của file nén này",
                CMD_ADD_TO_ARCHIVE => "Thêm file này vào 1 file .zip đã có sẵn",
                _ => return Err(E_INVALIDARG.into()),
            };
            // GCS_HELPTEXTW = GCS_HELPTEXTA(1) | GCS_UNICODE(4) = 5 — chỉ xử lý đúng trường
            // hợp này (text hiển thị ở status bar khi rê chuột qua mục menu); các `utype`
            // khác (GCS_VERBW, GCS_VALIDATEW, hay biến thể ANSI GCS_HELPTEXTA...) trả
            // `E_NOTIMPL` — không bắt buộc phải hỗ trợ đầy đủ để menu hoạt động đúng.
            // `pszname` khai kiểu `PSTR` cả trong trường hợp Unicode — quy ước lịch sử của
            // chính IContextMenu::GetCommandString (không có GetCommandStringW riêng), Explorer
            // tự diễn giải theo bit GCS_UNICODE trong `utype`, không theo kiểu tham số khai báo.
            const GCS_HELPTEXTW: u32 = 5;
            if utype != GCS_HELPTEXTW {
                return Err(E_NOTIMPL.into());
            }
            if pszname.is_null() || cchmax == 0 {
                return Err(E_INVALIDARG.into());
            }

            let max_chars = (cchmax as usize).saturating_sub(1);
            let text: Vec<u16> = help_text.encode_utf16().take(max_chars).chain(std::iter::once(0)).collect();
            let dest = pszname.0 as *mut u16;
            unsafe {
                std::ptr::copy_nonoverlapping(text.as_ptr(), dest, text.len());
            }
            Ok(())
        }))
        .unwrap_or(Err(E_FAIL.into()))
    }
}

/// Kiểm tra nội bộ (cùng crate, không qua `tests/`) — có quyền truy cập trực tiếp field
/// `selected` (module-private) để dựng đúng trạng thái "đã Initialize xong với 1 file archive
/// hợp lệ" mà không cần dựng cả 1 `IDataObject` giả (việc đó cần nhiều mã hơn đáng kể). Đi
/// qua đúng `ComObject::new` + gọi thẳng `IContextMenu_Impl::QueryContextMenu` — vẫn là vtable
/// COM thật, không phải gọi hàm Rust trần, giữ đúng giá trị kiểm chứng như test COM smoke ở
/// `tests/com_smoke_test.rs`.
#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::ComObject;
    use windows::Win32::UI::WindowsAndMessaging::{CreateMenu, GetMenuItemCount, GetMenuItemInfoW, GetSubMenu, MENUITEMINFOW};

    #[test]
    fn query_context_menu_inserts_one_popup_with_five_items_when_selected() {
        let obj: ComObject<VietzipContextMenu> = ComObject::new(VietzipContextMenu::default());
        *obj.selected.borrow_mut() = Some(PathBuf::from(r"C:\Users\test\Downloads\Photos.zip"));

        let hmenu = unsafe { CreateMenu() }.unwrap();
        let hr = obj.QueryContextMenu(hmenu, 0, 1, 100, 0);
        assert!(hr.is_ok(), "phải thành công khi đã có file được chọn: {hr:?}");
        assert_eq!(hr.0, COMMAND_COUNT as i32, "HRESULT phải trả về đúng offset tương đối (5), không cộng idCmdFirst");

        assert_eq!(unsafe { GetMenuItemCount(Some(hmenu)) }, 1, "chỉ 1 mục cha ở top-level (kiểu 7-Zip)");

        let submenu = unsafe { GetSubMenu(hmenu, 0) };
        assert!(!submenu.is_invalid(), "mục cha phải có 1 submenu con thật, không phải mục lá");
        assert_eq!(unsafe { GetMenuItemCount(Some(submenu)) }, COMMAND_COUNT as i32, "submenu phải có đúng 5 mục");
    }

    #[test]
    fn query_context_menu_adds_nothing_when_nothing_selected() {
        let obj: ComObject<VietzipContextMenu> = ComObject::new(VietzipContextMenu::default());
        // Không set `selected` — mô phỏng đúng trường hợp Initialize chưa từng chạy hoặc chạy
        // với lựa chọn không hợp lệ (nhiều file, hoặc không phải archive).
        let hmenu = unsafe { CreateMenu() }.unwrap();
        let hr = obj.QueryContextMenu(hmenu, 0, 1, 100, 0);
        assert!(hr.is_ok());
        assert_eq!(hr.0, 0, "offset trả về phải là 0 khi không thêm gì");
        assert_eq!(unsafe { GetMenuItemCount(Some(hmenu)) }, 0, "không được thêm gì khi chưa có file hợp lệ");
    }

    /// Xác nhận nhãn "Giải nén vào X" thật sự đọc được TỪ BÊN TRONG submenu (không chỉ đếm số
    /// lượng) — dùng `GetMenuItemInfoW` với `MIIM_STRING` để lấy chuỗi thật của 1 mục con.
    #[test]
    fn extract_subfolder_item_inside_submenu_has_real_filename() {
        use windows::Win32::UI::WindowsAndMessaging::MIIM_STRING;

        let obj: ComObject<VietzipContextMenu> = ComObject::new(VietzipContextMenu::default());
        *obj.selected.borrow_mut() = Some(PathBuf::from(r"C:\data\BaoCaoThang.zip"));

        let hmenu = unsafe { CreateMenu() }.unwrap();
        let _ = obj.QueryContextMenu(hmenu, 0, 1, 100, 0);
        let submenu = unsafe { GetSubMenu(hmenu, 0) };

        let mut buf = [0u16; 128];
        let mut info = MENUITEMINFOW {
            cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_STRING,
            dwTypeData: windows::core::PWSTR(buf.as_mut_ptr()),
            cch: buf.len() as u32,
            ..Default::default()
        };
        let ok = unsafe { GetMenuItemInfoW(submenu, CMD_EXTRACT_SUBFOLDER as u32, true, &mut info) };
        assert!(ok.is_ok());
        let label = String::from_utf16_lossy(&buf[..info.cch as usize]);
        assert!(
            label.contains("BaoCaoThang"),
            "mục 'Giải nén vào X' bên trong submenu phải chứa đúng tên file thật: {label:?}"
        );
    }
}
