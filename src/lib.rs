#![cfg(windows)]
#![no_std]

//! Rust wrapper of [ATL thunk](https://learn.microsoft.com/en-us/windows/win32/api/atlthunk/) type.

use ::windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use ::windows::Win32::System::Memory::AtlThunkData_t;
use ::windows::Win32::UI::WindowsAndMessaging::WNDPROC;
use core::ffi::c_void;
use core::mem;
use core::ptr::NonNull;

pub mod windows {
    pub use ::windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
}

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

type WindowProcedure = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

/// Rust wrapper of [ATL thunk](https://learn.microsoft.com/en-us/windows/win32/api/atlthunk/) type. It is used as
/// a [window procedure](https://learn.microsoft.com/en-us/windows/win32/winmsg/about-window-procedures) with associated
/// data.
pub struct AtlThunk {
    raw_thunk_ptr: NonNull<AtlThunkData_t>,
}

impl AtlThunk {
    /// Creates a new [`AtlThunk`] object. For more information, see document for
    /// [`AtlThunk_AllocateData`](<https://learn.microsoft.com/en-us/windows/win32/api/atlthunk/nf-atlthunk-atlthunk_allocatedata>).
    pub fn try_new() -> ::windows::core::Result<Self> {
        match NonNull::new(unsafe { AtlThunk_AllocateData() }) {
            None => Err(::windows::core::Error::from_thread()),
            Some(raw_thunk_ptr) => Ok(Self { raw_thunk_ptr }),
        }
    }

    /// Creates a new [`AtlThunk`] object from specified window procedure and associated first parameter value. For more
    /// information, see document for
    /// [`AtlThunk_AllocateData`](<https://learn.microsoft.com/en-us/windows/win32/api/atlthunk/nf-atlthunk-atlthunk_allocatedata>)
    /// and
    /// [`AtlThunk_InitData`](<https://learn.microsoft.com/en-us/windows/win32/api/atlthunk/nf-atlthunk-atlthunk_initdata>).
    pub fn try_new_with(window_procedure: WindowProcedure, first_parameter: HWND) -> ::windows::core::Result<Self> {
        let mut result = Self::try_new();

        if let Ok(thunk) = &mut result {
            thunk.set_data(window_procedure, first_parameter);
        }

        result
    }

    /// Returns a wrapped window procedure. The returned function pointer is only valid if the following conditions are
    /// met:
    ///
    /// - Associated data has been set through either [`AtlThunk::try_new_with`] or [`AtlThunk::set_data`].
    /// - The originating [`AtlThunk`] object has not been dropped.
    /// - There is no concurrent [`AtlThunk::set_data`] operating on the originating [`AtlThunk`] object.
    ///
    /// For more information, see document for
    /// [`AtlThunk_DataToCode`](<https://learn.microsoft.com/en-us/windows/win32/api/atlthunk/nf-atlthunk-atlthunk_datatocode>).
    #[inline(always)]
    pub fn as_window_procedure(&self) -> WindowProcedure {
        unsafe { AtlThunk_DataToCode(self.raw_thunk_ptr.as_ptr()).unwrap_unchecked() }
    }

    /// Updates the associated window procedure and data. For more information, see document for
    /// [`AtlThunk_InitData`](<https://learn.microsoft.com/en-us/windows/win32/api/atlthunk/nf-atlthunk-atlthunk_initdata>).
    #[inline(always)]
    pub fn set_data(&mut self, window_procedure: WindowProcedure, first_parameter: HWND) {
        unsafe {
            #[expect(clippy::transmutes_expressible_as_ptr_casts, reason = "by-design")]
            let procedure = mem::transmute::<WindowProcedure, *mut c_void>(window_procedure);

            let first_parameter = mem::transmute::<HWND, usize>(first_parameter);

            AtlThunk_InitData(self.raw_thunk_ptr.as_mut(), procedure, first_parameter);
        }
    }
}

impl Drop for AtlThunk {
    /// For more information, see document for
    /// [`AtlThunk_FreeData`](<https://learn.microsoft.com/en-us/windows/win32/api/atlthunk/nf-atlthunk-atlthunk_freedata>).
    #[inline(always)]
    fn drop(&mut self) {
        unsafe { AtlThunk_FreeData(self.raw_thunk_ptr.as_ptr()) };
    }
}

unsafe impl Send for AtlThunk {}
unsafe impl Sync for AtlThunk {}

#[cfg(test)]
mod tests {
    use super::AtlThunk;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

    #[test]
    fn test_thunk_try_new_with() {
        unsafe extern "system" fn callback_1(
            first_parameter: HWND,
            message: u32,
            w_param: WPARAM,
            l_param: LPARAM,
        ) -> LRESULT {
            assert_eq!(first_parameter.0 as usize, 2);
            assert_eq!(message, 3);
            assert_eq!(w_param.0, 5);
            assert_eq!(l_param.0, 7);

            LRESULT(11)
        }

        unsafe extern "system" fn callback_2(
            first_parameter: HWND,
            message: u32,
            w_param: WPARAM,
            l_param: LPARAM,
        ) -> LRESULT {
            assert_eq!(first_parameter.0 as usize, 13);
            assert_eq!(message, 17);
            assert_eq!(w_param.0, 19);
            assert_eq!(l_param.0, 23);

            LRESULT(29)
        }

        let mut thunk = AtlThunk::try_new_with(callback_1, HWND(2 as _)).unwrap();

        assert_eq!(
            unsafe { thunk.as_window_procedure()(HWND::default(), 3, WPARAM(5), LPARAM(7)) }.0,
            11,
        );

        thunk.set_data(callback_2, HWND(13 as _));

        assert_eq!(
            unsafe { thunk.as_window_procedure()(HWND::default(), 17, WPARAM(19), LPARAM(23)) }.0,
            29,
        );
    }
}
