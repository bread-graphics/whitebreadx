//               Copyright John Nunley, 2022.
// Distributed under the Boost Software License, Version 1.0.
//       (See accompanying file LICENSE or copy at
//         https://www.boost.org/LICENSE_1_0.txt)

#![cfg(feature = "xlib")]

use crate::{sync::Lazy, xcb_ffi::Connection};
use libc::{c_char, c_int};

#[cfg(feature = "dl")]
mod dynamic_link;
#[cfg(not(feature = "dl"))]
mod static_link;

/// FFI with `libX11`, using either static or dynamic linking.
///
/// # Safety
///
/// This trait is unsafe because it is not guaranteed that the underlying
/// library is safe.
#[allow(non_snake_case)]
pub(crate) unsafe trait X11Ffi {
    unsafe fn XOpenDisplay(&self, display: *const c_char) -> *mut XDisplay;
    unsafe fn XCloseDisplay(&self, display: *mut XDisplay) -> c_int;
    unsafe fn XDefaultScreen(&self, display: *mut XDisplay) -> c_int;
    unsafe fn XGetXCBConnection(&self, display: *mut XDisplay) -> *mut Connection;
    unsafe fn XInitThreads(&self) -> c_int;
}

#[repr(C)]
pub(crate) struct XDisplay {
    _opaque_type: [u8; 0],
}

#[cfg(not(feature = "dl"))]
type Impl = static_link::StaticLink;
#[cfg(feature = "dl")]
type Impl = dynamic_link::DynamicLink;

static XLIB: Lazy<Impl> = Lazy::new(|| {
    cfg_if::cfg_if! {
        if #[cfg(not(feature = "dl"))] {
            static_link::StaticLink
        } else {
            dynamic_link::DynamicLink::load()
        }
    }
});

pub(crate) fn xlib() -> &'static Impl {
    &*XLIB
}
