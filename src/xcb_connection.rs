//               Copyright John Nunley, 2022.
// Distributed under the Boost Software License, Version 1.0.
//       (See accompanying file LICENSE or copy at
//         https://www.boost.org/LICENSE_1_0.txt)

use crate::{
    cbox::CBox,
    extension_manager::ExtensionManager,
    sync::{call_once, mtx_lock, Mutex, OnceCell},
    xcb_ffi::{
        errors, flags, xcb, AuthInfo, Connection, GenericError, GenericEvent, Iovec,
        ProtocolRequest, VoidCookie, XcbFfi,
    },
};
use alloc::{sync::Arc, vec::Vec};
use breadx::{
    display::{Display, DisplayBase, DisplayFunctionsExt, RawReply, RawRequest},
    protocol::{xproto::Setup, Event, ReplyFdKind},
    x11_utils::TryParse,
    Error, Result,
};
use core::{
    alloc::Layout,
    mem::{self, MaybeUninit},
    ptr::{null, null_mut, slice_from_raw_parts_mut, NonNull},
    slice,
};
use cstr_core::CStr;
use libc::{c_int, c_void};

#[cfg(all(unix, feature = "to_socket"))]
use std::os::unix::io::{AsRawFd, RawFd};

/// A [`Display`] that acts as a wrapper around a `libxcb`
/// `xcb_connection_t`.
///
/// This acts identically to a standard `breadx` [`Display`],
/// except that it uses the `libxcb` connection type as its
/// internal transport. The primary advantage of this conversion
/// is that this display can be used in foreign libraries built
/// upon `libxcb`.
///
/// [`Display`]: breadx::display::Display
pub struct XcbDisplay {
    /// Pointer to the real connection object.
    connection: NonNull<Connection>,
    /// Whether we should call `xcb_disconnect` on drop.
    disconnect: bool,
    /// The converted setup associated with this connection.
    setup: OnceCell<Arc<Setup>>,
    /// Extension info manager.
    extension_manager: ExtensionManager,
    /// The set of all replies that will contain some number of FDs.
    has_fds: Mutex<HashSet<u64>>,
    /// The screen we're using.
    screen: usize,
}

unsafe impl Send for XcbDisplay {}
unsafe impl Sync for XcbDisplay {}

impl XcbDisplay {
    /// Connect to the X server.
    pub fn connect(display: Option<&CStr>) -> Result<XcbDisplay> {
        let mut screen = MaybeUninit::uninit();
        let display = display.map_or(null(), |display| display.as_ptr());

        let connection = unsafe { xcb().xcb_connect(display, screen.as_mut_ptr()) };

        Ok(unsafe { XcbDisplay::connected(connection, screen.assume_init() as usize)? })
    }

    /// Connect to the X11 server over the given auth address.
    pub fn connect_with_auth_info(
        display: Option<&CStr>,
        auth_name: &[u8],
        auth_data: &[u8],
    ) -> Result<XcbDisplay> {
        let mut screen = MaybeUninit::uninit();
        let mut auth_info = auth_info(auth_name, auth_data);
        let display = display.map_or(null(), |display| display.as_ptr());

        let connection = unsafe {
            xcb().xcb_connect_to_display_with_auth_info(
                display,
                &mut auth_info,
                screen.as_mut_ptr(),
            )
        };

        Ok(unsafe { XcbDisplay::connected(connection, screen.assume_init() as usize)? })
    }

    /// Connect to an FD.
    ///
    /// # Safety
    ///
    /// FD must be a valid file descriptor.
    pub unsafe fn connect_to_fd(
        fd: c_int,
        auth_name: &[u8],
        auth_data: &[u8],
        screen: usize,
    ) -> Result<XcbDisplay> {
        let mut auth_info = auth_info(auth_name, auth_data);

        let connection = unsafe { xcb().xcb_connect_to_fd(fd, &mut auth_info) };

        unsafe { XcbDisplay::connected(connection, screen) }
    }

    unsafe fn connected(ptr: *mut Connection, screen: usize) -> Result<Self> {
        assert!(!ptr.is_null());

        // check for a connection error
        let this = Self::from_ptr(ptr.cast(), true, screen as usize);

        if let Some(err) = this.take_error() {
            Err(err)
        } else {
            Ok(this)
        }
    }

    /// Wrap around an existing ptr.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid, non-null pointer to a `xcb_connection_t`. In addition
    /// `disconnect` should only be `true` if we logically own the connection.
    pub unsafe fn from_ptr(ptr: *mut c_void, disconnect: bool, screen: usize) -> XcbDisplay {
        let conn = NonNull::new_unchecked(ptr.cast());
        XcbDisplay {
            connection: conn,
            disconnect,
            setup: OnceCell::new(),
            extension_manager: ExtensionManager::new(),
            has_fds: Mutex::new(HashSet::with_hasher(Default::default())),
            screen,
        }
    }

    fn as_ptr(&self) -> *mut Connection {
        self.connection.as_ptr()
    }

    /// Get a pointer to the interior `libxcb` connection.
    pub fn as_raw_connection(&self) -> *mut c_void {
        self.as_ptr().cast()
    }

    /// Get the file descriptor for this FD.
    pub fn get_fd(&self) -> c_int {
        unsafe { xcb().xcb_get_file_descriptor(self.as_ptr()) }
    }

    /// Given a conn ptr, get the error.
    unsafe fn ptr_take_error(ptr: *mut Connection) -> Option<Error> {
        let error = unsafe { xcb().xcb_connection_has_error(ptr) };

        match error {
            0 => None,
            errors::XCB_CONN_ERROR => {
                // this is an I/O error, see if we can use I/O errors
                cfg_if::cfg_if! {
                    if #[cfg(feature = "std")] {
                        let io = std::io::Error::last_os_error();
                        Some(io.into())
                    } else {
                        Some(Error::make_msg(
                            "an unknown I/O error occurred"
                        ))
                    }
                }
            }
            errors::XCB_CONN_CLOSED_EXT_NOTSUPPORTED => {
                Some(Error::make_missing_extension("<unknown>"))
            }
            errors::XCB_CONN_CLOSED_MEM_INSUFFICIENT => {
                // standard Rust behavior when encountering an OOM
                // is to abort the program
                // we need a layout here for the error message
                // we don't know the exact one, but we can take an
                // educated guess
                let layout = Layout::from_size_align_unchecked(32, 4);

                alloc::alloc::handle_alloc_error(layout)
            }
            errors::XCB_CONN_CLOSED_REQ_LEN_EXCEED => {
                Some(Error::make_msg("request length exceeded"))
            }
            errors::XCB_CONN_CLOSED_PARSE_ERR => {
                Some(Error::make_msg("failed to parse server reply"))
            }
            errors::XCB_CONN_CLOSED_INVALID_SCREEN => Some(Error::make_msg("invalid screen")),
            errors::XCB_CONN_CLOSED_FDPASSING_FAILED => Some(Error::make_msg("failed to pass FD")),
            _ => Some(Error::make_msg("unknown error")),
        }
    }

    /// Convert our error into a `breadx` `Error`.
    pub fn take_error(&self) -> Option<Error> {
        unsafe { Self::ptr_take_error(self.as_ptr()) }
    }

    /// Take an error we may not have.
    pub fn take_maybe_error(&self) -> Error {
        match self.take_error() {
            Some(err) => err,
            None => Error::make_msg("no error"),
        }
    }

    /// Get the `Setup` associated with this type.
    pub fn get_setup(&self) -> &Arc<Setup> {
        call_once(&self.setup, || {
            // since xcb keeps its pointer types 1:1 equivalent with
            // the byte streams, we can just parse the setup as a
            // byte stream.
            let setup_ptr = unsafe { xcb().xcb_get_setup(self.as_ptr()) } as *mut u8 as *const u8;

            // figure out the length
            let header = unsafe { slice::from_raw_parts(setup_ptr, 8) };
            let xlen = u16::from_ne_bytes([header[6], header[7]]);
            let length = ((xlen as usize) * 4) + 8;

            // now, parse it
            let setup_slice = unsafe { slice::from_raw_parts(setup_ptr, length) };

            Setup::try_parse(setup_slice)
                .expect("xcb had invalid setup struct")
                .0
                .into()
        })
    }

    /// Generate a new XID.
    fn generate_xid_impl(&self) -> Result<u32> {
        let xid = unsafe { xcb().xcb_generate_id(self.as_ptr()) };

        if xid == -1i32 as u32 {
            Err(self.take_maybe_error())
        } else {
            Ok(xid)
        }
    }

    /// Get the maxmimum request length.
    fn maximum_request_length_impl(&self) -> u32 {
        unsafe { xcb().xcb_get_maximum_request_length(self.as_ptr()) }
    }

    fn synchronize_impl(&self) -> Result<()> {
        // send a checked no-op request
        let mut this = self;
        let cookie = this.no_operation()?;
        let seq = cookie.sequence();

        self.check_for_error_impl(seq)
    }

    /// Flush to the server.
    fn flush_impl(&self) -> Result<()> {
        let res = unsafe { xcb().xcb_flush(self.as_ptr()) };

        if res <= 0 {
            Err(self.take_maybe_error())
        } else {
            Ok(())
        }
    }

    unsafe fn parse_event(&self, event: *mut GenericEvent) -> Result<Event> {
        // inspect the header for info
        let header = event as *const GenericEvent as *const [u8; 32];
        let evbytes = event as *mut u8;
        let header = &*header;

        // tell if we're dealing with a generic event
        let mut length = 32;
        if header[0] & 0x7F == breadx::protocol::xproto::GE_GENERIC_EVENT {
            // read the length
            let xlen = u32::from_ne_bytes([header[4], header[5], header[6], header[7]]);
            let xlen = xlen as usize * 4;
            length += xlen;

            // xcb adds the sequence number for the event at 32 bytes,
            // discard it
            core::ptr::copy(evbytes.add(36), evbytes.add(32), xlen);
        }

        // create a CBox over the byte slice
        let event = slice_from_raw_parts_mut(evbytes, length);
        let event = unsafe { CBox::new(event) };

        // parse the event
        Event::parse(&event, &self.extension_manager).map_err(Error::make_parse_error)
    }

    /// Wait for an event.
    fn wait_for_event_impl(&self) -> Result<Event> {
        let event = unsafe { xcb().xcb_wait_for_event(self.as_ptr()) };

        let event = if event.is_null() {
            return Err(self.take_maybe_error());
        } else {
            event
        };

        unsafe { self.parse_event(event) }
    }

    /// Poll for an event.
    fn poll_for_event_impl(&self) -> Result<Option<Event>> {
        let event = unsafe { xcb().xcb_poll_for_event(self.as_ptr()) };

        let event = if event.is_null() {
            // tell if the null corresponds to an error
            if let Some(err) = self.take_error() {
                return Err(err);
            } else {
                return Ok(None);
            }
        } else {
            event
        };

        unsafe { self.parse_event(event) }.map(Some)
    }

    /// Send a request to the server.
    fn send_request_impl(&self, mut request: RawRequest) -> Result<u64> {
        // format the request
        let ext_opcode = request
            .extension()
            .map(|ext| {
                let mut this = self;
                match self.extension_manager.extension_code(&mut this, ext)? {
                    Some(code) => Ok(code),
                    None => Err(Error::make_missing_extension(ext)),
                }
            })
            .transpose()?;

        request.format(ext_opcode, self.maximum_request_length_impl() as usize)?;

        let variant = request.variant();
        let reply_has_fds = matches!(variant, ReplyFdKind::ReplyWithFDs);
        let check_reply = request.discard_mode().is_none();
        let (buf, fds) = request.mut_parts();

        let iov = (&mut buf[1..]).as_mut_ptr() as *mut Iovec;

        // determine protocol request
        let proto_request = ProtocolRequest {
            count: buf.len() - 1,
            extension: null_mut(),
            opcode: 0,
            isvoid: matches!(variant, ReplyFdKind::NoReply) as u8,
        };

        let mut sr_flags = flags::RAW;
        if check_reply {
            sr_flags |= flags::CHECKED;
        }
        if reply_has_fds {
            sr_flags |= flags::REPLY_HAS_FDS;
        }

        let seq = if fds.is_empty() {
            // no fds
            unsafe { xcb().xcb_send_request64(self.as_ptr(), sr_flags, iov, &proto_request) }
        } else {
            // we have fds
            let mut fds = mem::take(fds)
                .into_iter()
                .map(|fd| {
                    cfg_if::cfg_if! {
                        if #[cfg(all(unix, feature = "std"))] {
                            fd.into_raw_fd()
                        } else {
                            let _ = fd;
                            unreachable!()
                        }
                    }
                })
                .collect::<Vec<_>>();
            unsafe {
                xcb().xcb_send_request_with_fds64(
                    self.as_ptr(),
                    sr_flags,
                    iov,
                    &proto_request,
                    fds.len() as i32,
                    fds.as_mut_ptr(),
                )
            }

            // fds are manually closed by libxcb
        };

        // check for an error
        if seq == 0 {
            return Err(self.take_maybe_error());
        }

        // setup sequence number
        if reply_has_fds {
            mtx_lock(&self.has_fds).insert(seq);
        }

        Ok(seq)
    }

    #[cfg(unix)]
    unsafe fn extract_fds(&self, reply: &[u8], seq: u64) -> Vec<c_int> {
        // if the sequenc number is not in our set, return
        if !mtx_lock(&self.has_fds).remove(&seq) {
            return Vec::new();
        }

        let nfds = reply[1];
        let fd_ptr = (reply.as_ptr() as *const c_int).add(reply.len());
        let fd_slice = slice::from_raw_parts(fd_ptr, nfds as usize);

        fd_slice.into()
    }

    #[cfg(not(unix))]
    unsafe fn extract_fds(&self, _reply: &[u8], _seq: u64) -> Vec<c_int> {
        Vec::new()
    }

    unsafe fn wrap_error(&self, error: *mut GenericError) -> Error {
        use breadx::protocol::X11Error;

        let error_ptr = error as *mut [u8; 32];
        let error_boxed = unsafe { CBox::new(error_ptr) };

        // parse it
        X11Error::try_parse(&*error_boxed, &self.extension_manager)
            .map_or_else(Error::make_parse_error, Error::from)
    }

    /// Poll for a reply.
    fn poll_for_reply_impl(&self, seq: u64) -> Result<Option<XcbReply>> {
        // call poll_for_reply()
        let mut reply = null_mut();
        let mut error = null_mut();

        // poll for it
        let found =
            unsafe { xcb().xcb_poll_for_reply64(self.as_ptr(), seq, &mut reply, &mut error) };

        if found == 0 {
            return Ok(None);
        }

        // wrap the c_void into a reply type if we have it
        let reply = match (reply.is_null(), error.is_null()) {
            (true, true) => return Ok(None),
            (false, true) => {
                // got back a reply
                unsafe { wrap_reply(reply) }
            }
            (true, false) => {
                // got back an error
                return Err(unsafe { self.wrap_error(error) });
            }
            (false, false) => panic!("reply and error are both non-null"),
        };

        let fds = unsafe { self.extract_fds(reply.as_ref(), seq) };

        Ok(Some(XcbReply { reply, fds }))
    }

    // Wait for a reply.
    fn wait_for_reply_impl(&self, seq: u64) -> Result<XcbReply> {
        // call wait_for_reply()
        let mut error = null_mut();

        let reply = unsafe { xcb().xcb_wait_for_reply64(self.as_ptr(), seq, &mut error) };

        match (reply.is_null(), error.is_null()) {
            (true, true) => {
                // both null indicates an I/O error
                Err(self.take_maybe_error())
            }
            (false, true) => {
                // reply is non-null, return it
                let reply = unsafe { wrap_reply(reply) };
                let fds = unsafe { self.extract_fds(reply.as_ref(), seq) };

                Ok(XcbReply { reply, fds })
            }
            (true, false) => {
                // error is non-null
                Err(unsafe { self.wrap_error(error) })
            }
            (false, false) => {
                panic!("reply and error are both non-null")
            }
        }
    }

    fn check_for_error_impl(&self, seq: u64) -> Result<()> {
        let seq = VoidCookie { sequence: seq as _ };
        let err = unsafe { xcb().xcb_request_check(self.as_ptr(), seq) };

        if err.is_null() {
            return Ok(());
        }

        let err = unsafe { self.wrap_error(err) };
        Err(err)
    }
}

#[cfg(all(unix, feature = "to_socket"))]
impl XcbDisplay {
    /// Connect to an existing socket.
    ///
    /// # Safety
    ///
    /// `socket` must be a valid I/O socket.
    pub unsafe fn connect_to_socket(
        socket: impl AsRawFd,
        auth_name: &[u8],
        auth_data: &[u8],
        screen: usize,
    ) -> Result<Self> {
        // SAFETY: due to AsRawFd, we know socket is a valid socket
        // or do we? take another look once i/o safety lands
        unsafe { Self::connect_to_fd(socket.as_raw_fd(), auth_name, auth_data, screen) }
    }
}

#[cfg(all(unix, feature = "to_socket"))]
impl AsRawFd for XcbDisplay {
    fn as_raw_fd(&self) -> RawFd {
        self.as_ptr() as RawFd
    }
}

impl DisplayBase for XcbDisplay {
    fn setup(&self) -> &Arc<Setup> {
        self.get_setup()
    }

    fn default_screen_index(&self) -> usize {
        self.screen
    }

    fn poll_for_event(&mut self) -> Result<Option<Event>> {
        self.poll_for_event_impl()
    }

    fn poll_for_reply_raw(&mut self, seq: u64) -> Result<Option<RawReply>> {
        self.poll_for_reply_impl(seq).map(|o| o.map(Into::into))
    }
}

impl DisplayBase for &XcbDisplay {
    fn setup(&self) -> &Arc<Setup> {
        self.get_setup()
    }

    fn default_screen_index(&self) -> usize {
        self.screen
    }

    fn poll_for_event(&mut self) -> Result<Option<Event>> {
        self.poll_for_event_impl()
    }

    fn poll_for_reply_raw(&mut self, seq: u64) -> Result<Option<RawReply>> {
        self.poll_for_reply_impl(seq).map(|o| o.map(Into::into))
    }
}

impl Display for XcbDisplay {
    fn send_request_raw(&mut self, req: RawRequest<'_, '_>) -> Result<u64> {
        self.send_request_impl(req)
    }

    fn flush(&mut self) -> Result<()> {
        self.flush_impl()
    }

    fn generate_xid(&mut self) -> Result<u32> {
        self.generate_xid_impl()
    }

    fn maximum_request_length(&mut self) -> Result<usize> {
        Ok(self.maximum_request_length_impl() as usize)
    }

    fn synchronize(&mut self) -> Result<()> {
        self.synchronize_impl()
    }

    fn wait_for_event(&mut self) -> Result<Event> {
        self.wait_for_event_impl()
    }

    fn wait_for_reply_raw(&mut self, seq: u64) -> Result<RawReply> {
        self.wait_for_reply_impl(seq).map(Into::into)
    }

    fn check_for_error(&mut self, seq: u64) -> Result<()> {
        self.check_for_error_impl(seq)
    }
}

impl Display for &XcbDisplay {
    fn flush(&mut self) -> Result<()> {
        self.flush_impl()
    }

    fn generate_xid(&mut self) -> Result<u32> {
        self.generate_xid_impl()
    }

    fn maximum_request_length(&mut self) -> Result<usize> {
        Ok(self.maximum_request_length_impl() as usize)
    }

    fn send_request_raw(&mut self, req: RawRequest<'_, '_>) -> Result<u64> {
        self.send_request_impl(req)
    }

    fn synchronize(&mut self) -> Result<()> {
        self.synchronize_impl()
    }

    fn wait_for_event(&mut self) -> Result<Event> {
        self.wait_for_event_impl()
    }

    fn wait_for_reply_raw(&mut self, seq: u64) -> Result<RawReply> {
        self.wait_for_reply_impl(seq).map(Into::into)
    }

    fn check_for_error(&mut self, seq: u64) -> Result<()> {
        self.check_for_error_impl(seq)
    }
}

impl Drop for XcbDisplay {
    fn drop(&mut self) {
        if self.disconnect {
            unsafe {
                xcb().xcb_disconnect(self.as_ptr());
            }
        }
    }
}

unsafe fn wrap_reply(reply: *mut c_void) -> CBox<[u8]> {
    // determine the total length
    let header = unsafe { slice::from_raw_parts(reply as *mut u8 as *const u8, 32) };

    let length = u32::from_ne_bytes([header[4], header[5], header[6], header[7]]);

    // length is 32 plus four times the len
    let length = 32usize + (4 * (length as usize));
    let reply = slice_from_raw_parts_mut(reply as *mut u8, length);

    unsafe { CBox::new(reply) }
}

pub struct XcbReply {
    reply: CBox<[u8]>,
    fds: Vec<c_int>,
}

impl From<XcbReply> for RawReply {
    fn from(xcr: XcbReply) -> Self {
        let XcbReply { reply, fds } = xcr;

        let data = reply.clone_slice().into_boxed_slice();
        let fds = fds
            .into_iter()
            .map(|fd| {
                cfg_if::cfg_if! {
                    if #[cfg(all(unix, feature = "std"))] {
                        breadx::Fd::new(fd)
                    } else {
                        let _ = fd;
                        unreachable!()
                    }
                }
            })
            .collect::<Vec<breadx::Fd>>();

        RawReply::new(data, fds)
    }
}

fn auth_info(auth_name: &[u8], auth_data: &[u8]) -> AuthInfo {
    AuthInfo {
        namelen: auth_name.len() as _,
        name: auth_name.as_ptr() as *const _ as *mut _,
        datalen: auth_data.len() as _,
        data: auth_data.as_ptr() as *const _ as *mut _,
    }
}

/// HashSet type with a slight speedup in comparison to the standard library
/// implementation and the `ahash` crate used in the `breadx` crate.
///
/// Collision chances are higher, but given that the `has_fds` hash set
/// isn't used that often, it shouldn't come up.
type HashSet<T> = hashbrown::HashSet<T, core::hash::BuildHasherDefault<rustc_hash::FxHasher>>;
