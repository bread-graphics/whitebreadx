//               Copyright John Nunley, 2022.
// Distributed under the Boost Software License, Version 1.0.
//       (See accompanying file LICENSE or copy at
//         https://www.boost.org/LICENSE_1_0.txt)

use super::{X11Ffi, XDisplay};
use crate::xcb_ffi::Connection;
use libc::{c_char, c_int};
use libloading::Library;

pub(crate) struct DynamicLink {
    _xlib: Library,
    _xlib_xcb: Library,
    funcs: Funcs,
}

impl DynamicLink {
    pub(crate) fn load() -> Self {
        let xlib =
            unsafe { Library::new("libX11.so.6") }.expect("Unable to open libX11 dynamically");
        let xlib_xcb = unsafe { Library::new("libX11-xcb.so.1") }
            .expect("Unable to open libX11-xcb dynamically");

        let funcs = unsafe { Funcs::load(&xlib, &xlib_xcb) };

        Self {
            _xlib: xlib,
            _xlib_xcb: xlib_xcb,
            funcs,
        }
    }
}

unsafe impl X11Ffi for DynamicLink {
    unsafe fn XCloseDisplay(&self, display: *mut XDisplay) -> c_int {
        (self.funcs.XCloseDisplay)(display)
    }

    unsafe fn XDefaultScreen(&self, display: *mut XDisplay) -> c_int {
        (self.funcs.XDefaultScreen)(display)
    }

    unsafe fn XGetXCBConnection(&self, display: *mut XDisplay) -> *mut Connection {
        (self.funcs.XGetXCBConnection)(display)
    }

    unsafe fn XInitThreads(&self) -> c_int {
        (self.funcs.XInitThreads)()
    }

    unsafe fn XOpenDisplay(&self, display: *const libc::c_char) -> *mut XDisplay {
        (self.funcs.XOpenDisplay)(display)
    }
}

#[allow(non_snake_case)]
struct Funcs {
    XOpenDisplay: unsafe extern "C" fn(*const c_char) -> *mut XDisplay,
    XCloseDisplay: unsafe extern "C" fn(*mut XDisplay) -> c_int,
    XDefaultScreen: unsafe extern "C" fn(*mut XDisplay) -> c_int,
    XGetXCBConnection: unsafe extern "C" fn(*mut XDisplay) -> *mut Connection,
    XInitThreads: unsafe extern "C" fn() -> c_int,
}

impl Funcs {
    unsafe fn load(xlib: &Library, xlib_xcb: &Library) -> Self {
        Self {
            XOpenDisplay: {
                let symbol = concat!("XOpenDisplay\0").as_bytes();
                *(xlib
                    .get(symbol)
                    .expect(concat!("Could not find symbol: ", stringify!(XOpenDisplay))))
            },
            XCloseDisplay: {
                let symbol = concat!("XCloseDisplay\0").as_bytes();
                *(xlib.get(symbol).expect(concat!(
                    "Could not find symbol: ",
                    stringify!(XCloseDisplay)
                )))
            },
            XDefaultScreen: {
                let symbol = concat!("XDefaultScreen\0").as_bytes();
                *(xlib.get(symbol).expect(concat!(
                    "Could not find symbol: ",
                    stringify!(XDefaultScreen)
                )))
            },
            XGetXCBConnection: {
                let symbol = concat!("XGetXCBConnection\0").as_bytes();
                *(xlib_xcb.get(symbol).expect(concat!(
                    "Could not find symbol: ",
                    stringify!(XGetXCBConnection)
                )))
            },
            XInitThreads: {
                let symbol = concat!("XInitThreads\0").as_bytes();
                *(xlib
                    .get(symbol)
                    .expect(concat!("Could not find symbol: ", stringify!(XInitThreads))))
            },
        }
    }
}
