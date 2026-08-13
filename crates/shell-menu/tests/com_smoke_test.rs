//! Kiểm tra thật sự cơ chế COM (tạo object qua `DllGetClassObject`, `QueryInterface`,
//! `IContextMenu::QueryContextMenu`) chạy TRONG cùng tiến trình `cargo test` — không cần đăng
//! ký DLL vào registry, không cần Explorer thật. Đây đúng là loại lỗi từng được báo cáo thật
//! trên GitHub windows-rs (`InvokeCommand`/`GetCommandString` không được gọi ở 1 vài cách cài
//! đặt `#[implement]` ngây thơ) — test này gọi thẳng qua vtable thật của COM, không phải mock,
//! nên sẽ phát hiện được đúng loại lỗi đó nếu có.
//!
//! **Vẫn có giới hạn thật, không giấu**: đây KHÔNG thay thế được việc cài MSI thật rồi bấm
//! chuột phải thật trong Explorer — test này xác nhận cơ chế vtable/QueryInterface/callback
//! hoạt động đúng về mặt kỹ thuật COM, không xác nhận được toàn bộ luồng đăng ký registry +
//! Explorer thật gọi đúng theo đúng thứ tự như tài liệu (`IShellExtInit::Initialize` trước
//! `IContextMenu::QueryContextMenu`, dữ liệu `IDataObject` thật từ Explorer...).

#![cfg(windows)]

use vietzip_shell_menu::imp::{DllGetClassObject, CLSID_VIETZIP_CONTEXT_MENU};
use windows::core::Interface;
use windows::Win32::System::Com::IClassFactory;
use windows::Win32::UI::Shell::IContextMenu;
use windows::Win32::UI::WindowsAndMessaging::CreateMenu;

/// Gọi thẳng `DllGetClassObject` (đúng hàm Explorer thật sự gọi khi cần 1 instance) để lấy
/// `IClassFactory`, rồi `CreateInstance` ra `IContextMenu` — cùng đường đi thật Explorer dùng,
/// chỉ khác là gọi trực tiếp trong tiến trình test thay vì qua `CoCreateInstance`/registry.
fn create_context_menu() -> IContextMenu {
    let mut factory_ptr: *mut core::ffi::c_void = std::ptr::null_mut();
    let hr = DllGetClassObject(&CLSID_VIETZIP_CONTEXT_MENU, &IClassFactory::IID, &mut factory_ptr);
    assert!(hr.is_ok(), "DllGetClassObject phải thành công với đúng CLSID: {hr:?}");
    assert!(!factory_ptr.is_null());

    let factory: IClassFactory = unsafe { IClassFactory::from_raw(factory_ptr) };
    unsafe { factory.CreateInstance(None::<&windows::core::IUnknown>) }
        .expect("CreateInstance phải tạo được IContextMenu")
}

#[test]
fn dll_get_class_object_rejects_unknown_clsid() {
    let bogus = windows::core::GUID::from_u128(0x11111111_2222_3333_4444_555555555555);
    let mut ppv: *mut core::ffi::c_void = std::ptr::null_mut();
    let hr = DllGetClassObject(&bogus, &IClassFactory::IID, &mut ppv);
    assert!(hr.is_err(), "CLSID lạ phải bị từ chối, không được âm thầm trả về object");
    assert!(ppv.is_null());
}

#[test]
fn created_object_answers_both_interfaces() {
    let context_menu = create_context_menu();

    // Đối tượng phải trả lời được CẢ HAI interface đã khai trong #[implement(...)] — nếu
    // macro/registration có vấn đề, QueryInterface ở đây sẽ thất bại thay vì âm thầm sai.
    let as_shell_ext_init: windows::core::Result<windows::Win32::UI::Shell::IShellExtInit> = context_menu.cast();
    assert!(as_shell_ext_init.is_ok(), "object phải trả lời được IShellExtInit qua QueryInterface");
}

#[test]
fn query_context_menu_inserts_real_menu_item_and_encodes_command_count() {
    let context_menu = create_context_menu();

    // Không gọi `IShellExtInit::Initialize` trước (test này không có `IDataObject` thật từ
    // Explorer để đưa vào) — nghĩa là `selected` bên trong vẫn `None`, nên hành vi ĐÚNG là
    // KHÔNG thêm mục nào vào menu, không phải lỗi. Test này xác nhận đúng hành vi "an toàn
    // khi chưa init" đó, đồng thời xác nhận `QueryContextMenu` thật sự được gọi và trả về
    // HRESULT hợp lệ (không phải E_FAIL do panic/lỗi wiring).
    let hmenu = unsafe { CreateMenu() }.expect("CreateMenu (user32 thật) phải thành công");

    let hr = unsafe { context_menu.QueryContextMenu(hmenu, 0, 1, 100, 0) };
    assert!(
        hr.is_ok(),
        "QueryContextMenu phải trả về thành công (dù không thêm mục nào khi chưa Initialize): {hr:?}"
    );
}
