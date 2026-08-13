//! Lấy đường dẫn (các) file mà người dùng đang bấm chuột phải, từ `IDataObject` mà Explorer
//! đưa vào `IShellExtInit::Initialize`. Cơ chế chuẩn của Windows (không phải phát minh riêng):
//! yêu cầu định dạng `CF_HDROP` qua `IDataObject::GetData`, khoá `HGLOBAL` trả về bằng
//! `GlobalLock` để lấy con trỏ `HDROP`, rồi đọc từng đường dẫn bằng `DragQueryFileW` — xác
//! nhận đúng thứ tự gọi (kích thước trước, nội dung sau) qua mã nguồn thật của crate
//! `clipboard-win` (dùng chung API `DragQueryFileW`, dù nguồn khác là clipboard, không phải
//! `IDataObject` — cơ chế `DragQueryFileW` giống hệt nhau khi đã có `HDROP`).

use std::path::PathBuf;
use windows::Win32::System::Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL};
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_HDROP;
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

/// Đọc toàn bộ đường dẫn file trong `data_object` (định dạng `CF_HDROP`). Trả về rỗng nếu
/// `data_object` không mang dữ liệu `CF_HDROP` (vd người dùng bấm chuột phải trên nền trống,
/// không phải trên 1 file) — không phải lỗi, chỉ là "không áp dụng được cho lượt này".
pub(crate) fn extract_dropped_paths(data_object: &IDataObject) -> Vec<PathBuf> {
    let format = FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0 as u32,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };

    let Ok(medium) = (unsafe { data_object.GetData(&format) }) else {
        return Vec::new();
    };
    let hglobal = unsafe { medium.u.hGlobal };
    if hglobal.is_invalid() {
        return Vec::new();
    }

    let ptr = unsafe { GlobalLock(hglobal) };
    if ptr.is_null() {
        return Vec::new();
    }
    let hdrop = HDROP(ptr);

    let paths = read_all_paths(hdrop);

    let _ = unsafe { GlobalUnlock(hglobal) };
    paths
}

fn read_all_paths(hdrop: HDROP) -> Vec<PathBuf> {
    let count = unsafe { DragQueryFileW(hdrop, u32::MAX, None) };
    let mut paths = Vec::with_capacity(count as usize);

    for index in 0..count {
        let needed = unsafe { DragQueryFileW(hdrop, index, None) };
        if needed == 0 {
            continue;
        }
        // +1 cho ký tự NUL — khớp đúng quy ước Win32 (kích thước trả về ở lượt "chỉ hỏi độ
        // dài" KHÔNG tính NUL, nhưng buffer truyền vào lượt đọc thật cần đủ chỗ cho nó).
        let mut buffer = vec![0u16; needed as usize + 1];
        let written = unsafe { DragQueryFileW(hdrop, index, Some(&mut buffer)) };
        if written == 0 {
            continue;
        }
        buffer.truncate(written as usize);
        paths.push(PathBuf::from(String::from_utf16_lossy(&buffer)));
    }
    paths
}
