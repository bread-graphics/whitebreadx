// MIT/Apache2 License

#![cfg(not(feature = "dl"))]

use super::{
    AuthInfo, Connection, EventQueueKey, Extension, GenericError, GenericEvent, Iovec,
    ProtocolRequest, QueryExtensionReply, Setup, VoidCookie, XcbFfi,
};
use libc::{c_char, c_int, c_void};

pub(crate) struct StaticFfi;

unsafe impl XcbFfi for StaticFfi {
    unsafe fn xcb_connect(&self, display: *const c_char, screenp: *mut c_int) -> *mut Connection {
        xcb_connect(display, screenp)
    }

    unsafe fn xcb_connect_to_display_with_auth_info(
        &self,
        display: *const c_char,
        auth_info: *mut AuthInfo,
        screenp: *mut c_int,
    ) -> *mut Connection {
        xcb_connect_to_display_with_auth_info(display, auth_info, screenp)
    }

    unsafe fn xcb_connect_to_fd(&self, fd: c_int, auth_info: *mut AuthInfo) -> *mut Connection {
        xcb_connect_to_fd(fd, auth_info)
    }

    unsafe fn xcb_connection_has_error(&self, conn: *mut Connection) -> c_int {
        xcb_connection_has_error(conn)
    }

    unsafe fn xcb_disconnect(&self, conn: *mut Connection) {
        xcb_disconnect(conn)
    }

    unsafe fn xcb_flush(&self, conn: *mut Connection) -> c_int {
        xcb_flush(conn)
    }

    unsafe fn xcb_generate_id(&self, conn: *mut Connection) -> u32 {
        xcb_generate_id(conn)
    }

    unsafe fn xcb_get_file_descriptor(&self, conn: *mut Connection) -> c_int {
        xcb_get_file_descriptor(conn)
    }

    unsafe fn xcb_get_maximum_request_length(&self, conn: *mut Connection) -> u32 {
        xcb_get_maximum_request_length(conn)
    }

    unsafe fn xcb_get_setup(&self, conn: *mut Connection) -> *mut Setup {
        xcb_get_setup(conn)
    }

    unsafe fn xcb_poll_for_event(&self, conn: *mut Connection) -> *mut GenericEvent {
        xcb_poll_for_event(conn)
    }

    unsafe fn xcb_poll_for_reply64(
        &self,
        conn: *mut Connection,
        seq: u64,
        reply: *mut *mut c_void,
        error: *mut *mut GenericError,
    ) -> c_int {
        xcb_poll_for_reply64(conn, seq, reply, error)
    }

    unsafe fn xcb_send_request64(
        &self,
        conn: *mut Connection,
        flags: c_int,
        iov: *mut Iovec,
        request: *const ProtocolRequest,
    ) -> u64 {
        xcb_send_request64(conn, flags, iov, request)
    }

    unsafe fn xcb_send_request_with_fds64(
        &self,
        conn: *mut Connection,
        flags: c_int,
        iov: *mut Iovec,
        request: *const ProtocolRequest,
        num_fds: c_int,
        fds: *mut c_int,
    ) -> u64 {
        xcb_send_request_with_fds64(conn, flags, iov, request, num_fds, fds)
    }

    unsafe fn xcb_wait_for_event(&self, conn: *mut Connection) -> *mut GenericEvent {
        xcb_wait_for_event(conn)
    }

    unsafe fn xcb_wait_for_reply64(
        &self,
        conn: *mut Connection,
        seq: u64,
        error: *mut *mut GenericError,
    ) -> *mut c_void {
        xcb_wait_for_reply64(conn, seq, error)
    }

    unsafe fn xcb_request_check(
        &self,
        conn: *mut Connection,
        cookie: VoidCookie,
    ) -> *mut GenericError {
        xcb_request_check(conn, cookie)
    }
}

// actual import
#[link(name = "xcb")]
extern "C" {
    fn xcb_connect(display: *const c_char, screenp: *mut c_int) -> *mut Connection;
    fn xcb_connect_to_display_with_auth_info(
        display: *const c_char,
        auth_info: *mut AuthInfo,
        screenp: *mut c_int,
    ) -> *mut Connection;
    fn xcb_connect_to_fd(fd: c_int, auth_info: *mut AuthInfo) -> *mut Connection;
    fn xcb_get_file_descriptor(conn: *mut Connection) -> c_int;
    fn xcb_connection_has_error(conn: *mut Connection) -> c_int;
    fn xcb_disconnect(conn: *mut Connection);
    fn xcb_get_setup(conn: *mut Connection) -> *mut Setup;
    fn xcb_generate_id(conn: *mut Connection) -> u32;
    fn xcb_flush(conn: *mut Connection) -> c_int;
    fn xcb_get_maximum_request_length(conn: *mut Connection) -> u32;
    fn xcb_get_extension_data(
        conn: *mut Connection,
        ext: *mut Extension,
    ) -> *const QueryExtensionReply;
    fn xcb_wait_for_event(conn: *mut Connection) -> *mut GenericEvent;
    fn xcb_poll_for_event(conn: *mut Connection) -> *mut GenericEvent;
    fn xcb_wait_for_special_event(
        conn: *mut Connection,
        special_event: *mut EventQueueKey,
    ) -> *mut GenericEvent;
    fn xcb_poll_for_special_event(
        conn: *mut Connection,
        special_event: *mut EventQueueKey,
    ) -> *mut GenericEvent;
    fn xcb_register_for_special_xge(
        conn: *mut Connection,
        extension: *mut Extension,
        eid: u32,
        stamp: *mut u32,
    ) -> *mut EventQueueKey;
    fn xcb_unregister_for_special_event(conn: *mut Connection, special_event: *mut EventQueueKey);
    fn xcb_send_request64(
        conn: *mut Connection,
        flags: c_int,
        iov: *mut Iovec,
        request: *const ProtocolRequest,
    ) -> u64;
    fn xcb_send_request_with_fds64(
        conn: *mut Connection,
        flags: c_int,
        iov: *mut Iovec,
        request: *const ProtocolRequest,
        num_fds: c_int,
        fds: *mut c_int,
    ) -> u64;
    fn xcb_wait_for_reply64(
        conn: *mut Connection,
        seq: u64,
        error: *mut *mut GenericError,
    ) -> *mut c_void;
    fn xcb_poll_for_reply64(
        conn: *mut Connection,
        seq: u64,
        reply: *mut *mut c_void,
        error: *mut *mut GenericError,
    ) -> c_int;
    fn xcb_request_check(conn: *mut Connection, cookie: VoidCookie) -> *mut GenericError;
}
