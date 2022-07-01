//               Copyright John Nunley, 2022.
// Distributed under the Boost Software License, Version 1.0.
//       (See accompanying file LICENSE or copy at
//         https://www.boost.org/LICENSE_1_0.txt)

use super::{
    AuthInfo, Connection, GenericError, GenericEvent, Iovec, ProtocolRequest, Setup, VoidCookie,
    XcbFfi,
};
use libc::{c_char, c_int, c_void};
use libloading::Library;

pub(crate) struct DynamicFfi {
    _library: Library,
    funcs: Funcs,
}

impl DynamicFfi {
    pub(crate) fn load() -> Self {
        let path = "libxcb.so.1";

        let library = unsafe { Library::new(path) }.expect("Unable to open libxcb dynamically");

        let funcs = unsafe { Funcs::load(&library) };

        Self {
            _library: library,
            funcs,
        }
    }
}

macro_rules! define_funcs {
    (
        $($name: ident ($($arg: ident: $arg_ty: ty),*) -> $ret_ty: ty),*
    ) => {
        struct Funcs {
            $(
                $name: unsafe extern "C" fn($($arg_ty),*) -> $ret_ty,
            )*
        }

        impl Funcs {
            unsafe fn load(library: &Library) -> Self {
                Self {
                    $(
                    $name: {
                        let symbol = concat!(stringify!($name), "\0").as_bytes();
                        *(library
                            .get(symbol)
                            .expect(concat!("Could not find symbol: ", stringify!(name))))
                    },
                    )*
                }
            }

            $(
                unsafe fn $name(&self, $($arg: $arg_ty),*) -> $ret_ty {
                    unsafe {
                        (self.$name)($($arg),*)
                    }
                }
            )*
        }

        unsafe impl XcbFfi for DynamicFfi {
            $(
                unsafe fn $name(&self, $($arg: $arg_ty),*) -> $ret_ty {
                    self.funcs.$name($($arg),*)
                }
            )*
        }
    }
}

define_funcs! {
    xcb_connect(display: *const c_char, screenp: *mut c_int) -> *mut Connection,
    xcb_connect_to_display_with_auth_info(
        display: *const c_char,
        auth_info: *mut AuthInfo,
        screenp: *mut c_int
    ) -> *mut Connection,
    xcb_connect_to_fd(
        fd: c_int,
        auth_info: *mut AuthInfo
    ) -> *mut Connection,
    xcb_get_file_descriptor(conn: *mut Connection) -> c_int,
    xcb_connection_has_error(conn: *mut Connection) -> c_int,
    xcb_disconnect(conn: *mut Connection) -> (),
    xcb_flush(conn: *mut Connection) -> c_int,
    xcb_get_setup(conn: *mut Connection) -> *mut Setup,
    xcb_generate_id(conn: *mut Connection) -> u32,
    xcb_get_maximum_request_length(conn: *mut Connection) -> u32,
    xcb_wait_for_event(conn: *mut Connection) -> *mut GenericEvent,
    xcb_poll_for_event(conn: *mut Connection) -> *mut GenericEvent,
    xcb_send_request64(
        conn: *mut Connection,
        flags: c_int,
        iov: *mut Iovec,
        request: *const ProtocolRequest
    ) -> u64,
    xcb_send_request_with_fds64(
        conn: *mut Connection,
        flags: c_int,
        iov: *mut Iovec,
        request: *const ProtocolRequest,
        num_fds: c_int,
        fds: *mut c_int
    ) -> u64,
    xcb_wait_for_reply64(
        conn: *mut Connection,
        seq: u64,
        error: *mut *mut GenericError
    ) -> *mut c_void,
    xcb_poll_for_reply64(
        conn: *mut Connection,
        seq: u64,
        reply: *mut *mut c_void,
        error: *mut *mut GenericError
    ) -> c_int,
    xcb_request_check(
        conn: *mut Connection,
        request: VoidCookie
    ) -> *mut GenericError
}
