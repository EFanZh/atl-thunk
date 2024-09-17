#![cfg(windows)]
#![no_std]

//! Rust wrapper of [ATL thunk](https://learn.microsoft.com/en-us/windows/win32/api/atlthunk/) type.

use core::ffi::c_void;
use core::mem;
use core::ptr::NonNull;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Memory::AtlThunkData_t;
use windows::Win32::UI::WindowsAndMessaging::WNDPROC;

#[cfg_attr(
    target_arch = "x86",
    link(
        name = "atlthunk.dll",
        kind = "raw-dylib",
        modifiers = "+verbatim",
        import_name_type = "undecorated"
    )
)]
#[cfg_attr(
    not(target_arch = "x86"),
    link(name = "atlthunk.dll", kind = "raw-dylib", modifiers = "+verbatim")
)]
extern "system" {
    /// <https://learn.microsoft.com/en-us/windows/win32/api/atlthunk/nf-atlthunk-atlthunk_allocatedata>.
    fn AtlThunk_AllocateData() -> *mut AtlThunkData_t;

    /// <https://learn.microsoft.com/en-us/windows/win32/api/atlthunk/nf-atlthunk-atlthunk_datatocode>.
    fn AtlThunk_DataToCode(thunk: *mut AtlThunkData_t) -> WNDPROC;

    /// <https://learn.microsoft.com/en-us/windows/win32/api/atlthunk/nf-atlthunk-atlthunk_freedata>.
    fn AtlThunk_FreeData(thunk: *mut AtlThunkData_t);

    /// <https://learn.microsoft.com/en-us/windows/win32/api/atlthunk/nf-atlthunk-atlthunk_initdata>.
    fn AtlThunk_InitData(thunk: *mut AtlThunkData_t, proc: *mut c_void, first_parameter: usize);
}

type RawWindowProcedure<T> = unsafe extern "system" fn(T, u32, WPARAM, LPARAM) -> LRESULT;

/// Rust wrapper of [ATL thunk](https://learn.microsoft.com/en-us/windows/win32/api/atlthunk/) type. It is used as
/// a [window procedure](https://learn.microsoft.com/en-us/windows/win32/winmsg/about-window-procedures) with associated
/// data.
pub struct AtlThunk {
    thunk: NonNull<AtlThunkData_t>,
}

impl AtlThunk {
    /// Creates a new [`AtlThunk`] object.
    #[inline(always)]
    pub fn try_new(procedure: RawWindowProcedure<usize>, first_parameter: usize) -> windows::core::Result<Self> {
        let thunk = unsafe { AtlThunk_AllocateData() };

        let Some(thunk) = NonNull::new(thunk) else {
            return Err(windows::core::Error::from_win32());
        };

        let mut result = Self { thunk };

        result.set_data(procedure, first_parameter);

        Ok(result)
    }

    /// Returns a wrapped window procedure. The returned function pointer is only valid before the corresponding
    /// [`AtlThunk`] object drops.
    #[inline(always)]
    pub fn as_raw_window_procedure(&self) -> RawWindowProcedure<HWND> {
        unsafe { AtlThunk_DataToCode(self.thunk.as_ptr()).unwrap_unchecked() }
    }

    /// Updates the associated window procedure and data.
    #[inline(always)]
    pub fn set_data(&mut self, procedure: RawWindowProcedure<usize>, first_parameter: usize) {
        unsafe {
            #[expect(clippy::transmutes_expressible_as_ptr_casts, reason = "by-design")]
            let procedure = mem::transmute::<RawWindowProcedure<usize>, *mut c_void>(procedure);

            AtlThunk_InitData(self.thunk.as_mut(), procedure, first_parameter);
        }
    }
}

impl Drop for AtlThunk {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe { AtlThunk_FreeData(self.thunk.as_ptr()) };
    }
}

unsafe impl Send for AtlThunk {}
unsafe impl Sync for AtlThunk {}

#[cfg(test)]
mod tests {
    use super::AtlThunk;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

    #[test]
    fn test_thunk() {
        unsafe extern "system" fn callback_1(
            first_parameter: usize,
            message: u32,
            w_param: WPARAM,
            l_param: LPARAM,
        ) -> LRESULT {
            assert_eq!(first_parameter, 2);
            assert_eq!(message, 3);
            assert_eq!(w_param.0, 5);
            assert_eq!(l_param.0, 7);

            LRESULT(11)
        }

        unsafe extern "system" fn callback_2(
            first_parameter: usize,
            message: u32,
            w_param: WPARAM,
            l_param: LPARAM,
        ) -> LRESULT {
            assert_eq!(first_parameter, 13);
            assert_eq!(message, 17);
            assert_eq!(w_param.0, 19);
            assert_eq!(l_param.0, 23);

            LRESULT(29)
        }

        let mut thunk = AtlThunk::try_new(callback_1, 2).unwrap();

        assert_eq!(
            unsafe { thunk.as_raw_window_procedure()(HWND::default(), 3, WPARAM(5), LPARAM(7)) }.0,
            11,
        );

        thunk.set_data(callback_2, 13);

        assert_eq!(
            unsafe { thunk.as_raw_window_procedure()(HWND::default(), 17, WPARAM(19), LPARAM(23)) }.0,
            29,
        );
    }
}
