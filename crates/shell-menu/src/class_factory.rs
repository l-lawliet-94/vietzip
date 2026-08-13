//! `IClassFactory` tối giản — Explorer gọi `DllGetClassObject` để lấy 1 factory, rồi gọi
//! `CreateInstance` trên factory đó mỗi khi cần 1 instance mới của `VietzipContextMenu`
//! (thường là 1 lần mỗi lượt bấm chuột phải). Không hỗ trợ aggregation (`punkouter` phải là
//! `None`) — Explorer không cần tới cơ chế đó cho context menu handler, và không hỗ trợ giữ
//! đơn giản hoá, đúng tinh thần tối giản đã áp dụng xuyên suốt dự án này (NFR-11).

use crate::context_menu::VietzipContextMenu;
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use windows::core::{implement, BOOL, ComObject, IUnknown, IUnknownImpl, Result, GUID};
use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_FAIL};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};

#[implement(IClassFactory)]
pub struct VietzipClassFactory;

impl IClassFactory_Impl for VietzipClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: windows::core::Ref<'_, IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> Result<()> {
        catch_unwind(AssertUnwindSafe(|| {
            if ppvobject.is_null() {
                return Err(windows::Win32::Foundation::E_INVALIDARG.into());
            }
            unsafe { *ppvobject = std::ptr::null_mut() };
            if punkouter.is_some() {
                return Err(CLASS_E_NOAGGREGATION.into());
            }
            if riid.is_null() {
                return Err(windows::Win32::Foundation::E_INVALIDARG.into());
            }

            let obj: ComObject<VietzipContextMenu> = ComObject::new(VietzipContextMenu::default());
            // `QueryInterface` tự AddRef khi thành công (đúng ngữ nghĩa COM chuẩn) — `obj` ở
            // đây drop ngay sau đó chỉ trả refcount về đúng 1 tham chiếu mà caller sở hữu,
            // không rò rỉ, không giải phóng sớm.
            unsafe { obj.QueryInterface(riid, ppvobject).ok() }
        }))
        .unwrap_or(Err(E_FAIL.into()))
    }

    fn LockServer(&self, _flock: BOOL) -> Result<()> {
        // Không theo dõi lock count — DLL chỉ tồn tại trong lúc Explorer đang dùng, không có
        // trạng thái nào cần giữ sống lâu hơn qua LockServer(TRUE), an toàn khi bỏ qua.
        Ok(())
    }
}
