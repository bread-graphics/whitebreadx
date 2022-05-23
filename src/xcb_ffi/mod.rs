// MIT/Apache2 License

use alloc::boxed::Box;
use core::ptr::null_mut;
use libc::{c_char, c_int, c_uint, c_void};
use once_cell::sync::Lazy;

/// A trait for FFI with `libxcb`, using either static or dynamic linking.
pub(crate) unsafe trait XcbFfi {
    // connecting
    unsafe fn xcb_connect(&self, display: *const c_char, screenp: *mut c_int) -> *mut Connection;
    unsafe fn xcb_connect_to_display_with_auth_info(
        &self,
        display: *const c_char,
        auth_info: *mut AuthInfo,
        screenp: *mut c_int,
    ) -> *mut Connection;
    unsafe fn xcb_connect_to_fd(&self, fd: c_int, auth_info: *mut AuthInfo) -> *mut Connection;

    // utilities
    unsafe fn xcb_get_file_descriptor(&self, conn: *mut Connection) -> c_int;
    unsafe fn xcb_connection_has_error(&self, conn: *mut Connection) -> c_int;
    unsafe fn xcb_disconnect(&self, conn: *mut Connection);
    unsafe fn xcb_get_setup(&self, conn: *mut Connection) -> *mut Setup;
    unsafe fn xcb_generate_id(&self, conn: *mut Connection) -> u32;
    unsafe fn xcb_flush(&self, conn: *mut Connection) -> c_int;
    unsafe fn xcb_get_maximum_request_length(&self, conn: *mut Connection) -> c_int;

    // events
    unsafe fn xcb_wait_for_event(&self, conn: *mut Connection) -> *mut GenericEvent;
    unsafe fn xcb_poll_for_event(&self, conn: *mut Connection) -> *mut GenericEvent;
    unsafe fn xcb_wait_for_special_event(
        &self,
        conn: *mut Connection,
        special_event: *mut EventQueueKey
    ) -> *mut GenericEvent;
    unsafe fn xcb_poll_for_special_event(
        &self,
        conn: *mut Connection,
        special_event: *mut EventQueueKey
    ) -> *mut GenericEvent;
    unsafe fn xcb_register_for_special_xge(
        &self,
        conn: *mut Connection,
        extension: *mut Extension,
        eid: u32,
        stamp: *mut u32,
    ) -> *mut EventQueueKey;
    unsafe fn xcb_unregister_for_special_event(
        &self,
        conn: *mut Connection,
        special_event: *mut EventQueueKey
    );

    // requests api
    unsafe fn xcb_send_request64(
        &self,
        conn: *mut Connection,
        flags: c_int,
        iov: *mut Iovec,
        request: *const ProtocolRequest
    ) -> u64;
    unsafe fn xcb_send_request_with_fds64(
        &self,
        conn: *mut Connection,
        flags: c_int,
        iov: *mut Iovec,
        request: *const ProtocolRequest,
        num_fds: c_int,
        fds: *mut c_int,
    ) -> u64;
    unsafe fn xcb_wait_for_reply64(
        &self,
        conn: *mut Connection,
        seq: u64,
        error: *mut *mut GenericError,
    ) -> *mut c_void;
    unsafe fn xcb_poll_for_reply64(
        &self,
        conn: *mut Connection,
        seq: u64,
        reply: *mut *mut c_void,
        error: *mut *mut GenericError,
    ) -> c_int;

    /// Get the extension object for the given extension name.
    fn extension(
        &self,
        name: &str,
    ) -> Option<*mut Extension>;
}

/// Opaque type for the `libxcb` connection.
#[repr(C)]
pub(crate) struct Connection {
    _opaque_type: (),
}

/// Type for authorization info.
#[repr(C)]
pub(crate) struct AuthInfo {
    pub(crate) namelen: c_int,
    pub(crate) name: *mut c_char,
    pub(crate) datalen: c_int,
    pub(crate) data: *mut c_char,
}

/// XCB-side setup struct.
#[repr(C)]
pub(crate) struct Setup {
    // todo
}

/// XCB-side event repr.
#[repr(C)]
pub(crate) struct GenericEvent {
    _opaqe_type: (),
}

/// Special event queue key.
#[repr(C)]
pub(crate) struct EventQueueKey {
    // todo
}

/// Extension type.
#[repr(C)]
pub(crate) struct Extension {
    _opaque_type: (),
}

#[cfg(unix)]
pub(crate) use libc::iovec as Iovec;
#[cfg(not(unix))]
#[repr(C)]
pub(crate) struct Iovec {
    pub(crate) iov_base: *mut c_void,
    pub(crate) iov_len: c_int,
}

pub(crate) fn empty_iov() -> Iovec {
    Iovec {
        iov_base: null_mut(),
        iov_len: 0,
    }
}

/// Protocol request.
#[repr(C)]
pub(crate) struct ProtocolRequest {
    count: usize,
    extension: *mut Extension,
    opcode: u8,
    pub(crate) isvoid: u8,
}

/// X11 error that may occur.
#[repr(C)]
pub(crate) struct GenericError {
    // todo
}

/// Global object used to make `libxcb` calls.
static XCB: Lazy<Box<dyn XcbFfi + Send + Sync + 'static>> = Lazy::new(|| {
    todo!()
});

pub(crate) fn xcb() -> &'static dyn XcbFfi {
    &**XCB
} 

pub(crate) mod flags {
    use libc::c_int;

    pub(crate) const CHECKED: c_int = 0;
    pub(crate) const REPLY_HAS_FDS: c_int = 1;
}