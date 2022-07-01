//               Copyright John Nunley, 2022.
// Distributed under the Boost Software License, Version 1.0.
//       (See accompanying file LICENSE or copy at
//         https://www.boost.org/LICENSE_1_0.txt)

#![cfg(not(feature = "dl"))]

use super::{X11Ffi, XDisplay};
use crate::xcb_ffi::Connection;
use libc::{c_char, c_int};

pub(crate) struct StaticLink;

unsafe impl X11Ffi for StaticLink {
    unsafe fn XOpenDisplay(&self, display: *const c_char) -> *mut XDisplay {
        XOpenDisplay(display)
    }

    unsafe fn XCloseDisplay(&self, display: *mut XDisplay) -> c_int {
        XCloseDisplay(display)
    }

    unsafe fn XDefaultScreen(&self, display: *mut XDisplay) -> c_int {
        XDefaultScreen(display)
    }

    unsafe fn XGetXCBConnection(&self, display: *mut XDisplay) -> *mut Connection {
        XGetXCBConnection(display)
    }

    unsafe fn XInitThreads(&self) -> c_int {
        XInitThreads()
    }
}

#[link(name = "X11")]
extern "C" {
    fn XOpenDisplay(display: *const c_char) -> *mut XDisplay;
    fn XCloseDisplay(display: *mut XDisplay) -> c_int;
    fn XDefaultScreen(display: *mut XDisplay) -> c_int;
    fn XInitThreads() -> c_int;
}

#[link(name = "X11-xcb")]
extern "C" {
    fn XGetXCBConnection(display: *mut XDisplay) -> *mut Connection;
}
