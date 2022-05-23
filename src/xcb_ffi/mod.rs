// MIT/Apache2 License

use alloc::boxed::Box;
use core::ptr::null_mut;
use libc::{c_char, c_int, c_uint, c_void};
use once_cell::sync::Lazy;

mod static_link;

/// A trait for FFI with `libxcb`, using either static or dynamic linking.
#[allow(clippy::missing_safety_doc)]
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
    unsafe fn xcb_get_maximum_request_length(&self, conn: *mut Connection) -> u32;
    unsafe fn xcb_get_extension_data(
        &self,
        conn: *mut Connection,
        ext: *mut Extension,
    ) -> *const QueryExtensionReply;

    // events
    unsafe fn xcb_wait_for_event(&self, conn: *mut Connection) -> *mut GenericEvent;
    unsafe fn xcb_poll_for_event(&self, conn: *mut Connection) -> *mut GenericEvent;
    unsafe fn xcb_wait_for_special_event(
        &self,
        conn: *mut Connection,
        special_event: *mut EventQueueKey,
    ) -> *mut GenericEvent;
    unsafe fn xcb_poll_for_special_event(
        &self,
        conn: *mut Connection,
        special_event: *mut EventQueueKey,
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
        special_event: *mut EventQueueKey,
    );

    // requests api
    unsafe fn xcb_send_request64(
        &self,
        conn: *mut Connection,
        flags: c_int,
        iov: *mut Iovec,
        request: *const ProtocolRequest,
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
    unsafe fn xcb_request_check(&self, conn: *mut Connection, cookie: VoidCookie) -> *mut GenericError;
}

/// Opaque type for the `libxcb` connection.
#[repr(C)]
pub(crate) struct Connection {
    _opaque_type: [u8; 0],
}

#[repr(C)]
pub(crate) struct VoidCookie {
    pub(crate) sequence: c_uint,
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
    _opaque_type: [u8; 0],
}

/// XCB-side event repr.
#[repr(C)] // todo
pub(crate) struct EventQueueKey {
    _opaque_type: [u8; 0],
}

/// Extension type.
#[repr(C)]
pub(crate) struct Extension {
    _opaque_type: [u8; 0],
}

/// Query extension reply.
#[repr(C)]
pub(crate) struct QueryExtensionReply {
    response_type: u8,
    pad0: u8,
    sequence: u16,
    length: u32,
    pub(crate) present: u8,
    pub(crate) major_opcode: u8,
    pub(crate) first_event: u8,
    pub(crate) first_error: u8,
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
    pub(crate) count: usize,
    pub(crate) extension: *mut Extension,
    pub(crate) opcode: u8,
    pub(crate) isvoid: u8,
}

/// X11 error that may occur.
#[repr(C)]
pub(crate) struct GenericError {
    _opaque_type: [u8; 0],
}

/// X11 event.
#[repr(C)]
pub(crate) struct GenericEvent {
    _opaque_type: [u8; 0],
}

/// Global object used to make `libxcb` calls.
static XCB: Lazy<Box<dyn XcbFfi + Send + Sync + 'static>> = Lazy::new(|| {
    cfg_if::cfg_if! {
        if #[cfg(feature = "dl")] {
            todo!()
        } else {
            Box::new(static_link::StaticFfi)
        }
    }
});

pub(crate) fn xcb() -> &'static dyn XcbFfi {
    &**XCB
}

pub(crate) mod flags {
    use libc::c_int;

    pub(crate) const RAW: c_int = 2;
    pub(crate) const CHECKED: c_int = 1;
    pub(crate) const REPLY_HAS_FDS: c_int = 8;
}

pub(crate) mod errors {
    use libc::c_int;

    pub(crate) const XCB_CONN_ERROR: c_int = 1;

    pub(crate) const XCB_CONN_CLOSED_EXT_NOTSUPPORTED: c_int = 2;

    pub(crate) const XCB_CONN_CLOSED_MEM_INSUFFICIENT: c_int = 3;

    pub(crate) const XCB_CONN_CLOSED_REQ_LEN_EXCEED: c_int = 4;

    pub(crate) const XCB_CONN_CLOSED_PARSE_ERR: c_int = 5;

    pub(crate) const XCB_CONN_CLOSED_INVALID_SCREEN: c_int = 6;

    pub(crate) const XCB_CONN_CLOSED_FDPASSING_FAILED: c_int = 7;
}