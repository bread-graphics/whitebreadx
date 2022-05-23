// MIT/Apache2 License

use crate::{
    cbox::CBox,
    sync::{Mutex, RwLock},
    xcb_ffi::{GenericEvent, flags, xcb, Connection, AuthInfo, GenericError, Iovec, empty_iov, ProtocolRequest, EventQueueKey}
};
use alloc::vec::Vec;
use breadx::{Error, Result, protocol::{Event, xproto::Setup, ReplyFdKind}, display::{RawReply, RawRequest}};
use core::{mem::{MaybeUninit, ManuallyDrop}, slice, ptr::{slice_from_raw_parts_mut, null, null_mut, NonNull}};
use cstr_core::CStr;
use libc::{c_int, c_void};
use once_cell::sync::OnceCell;

/// A wrapper around the `libxcb` connection.
pub(crate) struct XcbConnection {
    /// Pointer to the real connection object.
    connection: NonNull<Connection>,
    /// Whether we should call `xcb_disconnect` on drop.
    disconnect: bool,
    /// The converted setup associated with this connection.
    setup: OnceCell<Setup>,
    /// Extension info manager.
    extension_manager: RwLock<ExtensionManager>,
    /// The set of all replies that will contain some number of FDs.
    has_fds: Mutex<HashSet<u64>>,
}

unsafe impl Send for XcbConnection {}
unsafe impl Sync for XcbConnection {}

impl XcbConnection {
    /// Connect to the X server.
    pub(crate) fn connect(
        display: Option<&CStr>,
    ) -> Result<(XcbConnection, c_int)> {
        let mut screen = MaybeUninit::uninit();
        let display = display.map_or(null(), |display| display.as_ptr());

        let connection = unsafe {
            xcb().xcb_connect(display, screen.as_mut_ptr())
        };

        Ok(unsafe {
            (
                XcbConnection::connected(connection)?,
                screen.assume_init()
            )
        })
    }

    /// Connect to the X11 server over the given auth address.
    pub(crate) fn connect_with_auth_info(
        display: Option<&CStr>,
        auth_name: &[u8],
        auth_data: &[u8],
    ) -> Result<(XcbConnection, c_int)> {
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

        Ok(unsafe {
            (
                XcbConnection::connected(connection)?,
                screen.assume_init()
            )
        })
    }

    /// Connect to an FD.
    /// 
    /// # Safety
    /// 
    /// FD must be a valid file descriptor.
    pub(crate) unsafe fn connect_to_fd(
        fd: c_int,
        auth_name: &[u8],
        auth_data: &[u8],
    ) -> Result<XcbConnection> {
        let mut auth_info = auth_info(auth_name, auth_data);

        let connection = unsafe {
            xcb().xcb_connect_to_fd(fd, &mut auth_info)
        };

        unsafe { XcbConnection::connected(connection) }
    }

    unsafe fn connected(
        ptr: *mut Connection,
    ) -> Result<Self> {
        assert!(!ptr.is_null());

        // check for a connection error
        let this = Self::from_ptr(ptr, true);
        
        if let Some(err) = this.take_error() {
            Err(err)
        } else {
            Ok(this)
        }
    }

    /// Wrap around an existing ptr.
    pub(crate) unsafe fn from_ptr(
        ptr: *mut Connection,
        disconnect: bool,
    ) -> XcbConnection {
        XcbConnection {
            connection: NonNull::new_unchecked(ptr),
            disconnect,
            setup: OnceCell::new(),
            extension_manager: RwLock::new(ExtensionManager::default()),
            has_fds: Mutex::new(HashSet::with_hasher(Default::default())),
        }
    }

    pub(crate) fn as_ptr(&self) -> *mut Connection {
        self.connection.as_ptr()
    }

    /// Get the file descriptor for this FD.
    pub(crate) fn get_fd(&self) -> c_int {
        unsafe {
            xcb().xcb_get_file_descriptor(self.as_ptr())
        }
    }

    /// If this type has an error, return the code.
    fn has_error(&self) -> c_int {
        unsafe {
            xcb().xcb_connection_has_error(self.as_ptr())
        }
    }

    /// Convert our error into a `breadx` `Error`.
    pub(crate) fn take_error(&self) -> Option<Error> {
        let err = self.has_error();

        match err {
            0 => None,
            _ => todo!(),
        }
    }

    /// Take an error we may not have.
    pub(crate) fn take_maybe_error(&self) -> Error {
        match self.take_error() {
            Some(err) => err,
            None => Error::make_msg("no error"),
        }
    }

    /// Get the `Setup` associated with this type.
    pub(crate) fn get_setup(&self) -> &Setup {
        self.setup.get_or_init(|| {
            todo!()
        })
    }

    /// Generate a new XID.
    pub(crate) fn generate_xid(&self) -> Result<u32> {
        let xid = unsafe {
            xcb().xcb_generate_id(self.as_ptr())
        };

        if xid == -1i32 as u32 {
            Err(self.take_maybe_error())
        } else {
            Ok(xid)
        }
    }

    /// Flush to the server.
    pub(crate) fn flush(&self) -> Result<()> {
        let res = unsafe {
            xcb().xcb_flush(self.as_ptr())
        };

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
            let xlen = u32::from_ne_bytes(
                [header[4], header[5], header[6], header[7]]
            );
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
        Event::parse(
            &event,
            &*self.extension_manager.read()
        ).map_err(|err| Error::make_parse_error(err))
    }

    /// Wait for an event.
    pub(crate) fn wait_for_event(&self) -> Result<Event> {
        let event = unsafe {
            xcb().xcb_wait_for_event(self.as_ptr())
        };

        let event = if event.is_null() {
            return Err(self.take_error().unwrap_or_else(|| Error::make_msg("Failed to wait for event")));
        } else {
            event
        };

        unsafe { self.parse_event(event) }
    }

    /// Poll for an event.
    pub(crate) fn poll_for_event(&self) -> Result<Option<Event>> {
        let event = unsafe {
            xcb().xcb_poll_for_event(self.as_ptr())
        };

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

    /// Wait for a special event.
    pub(crate) fn wait_for_special_event(&self, evkey: &SpecialEvent) -> Result<Event> {
        let event = unsafe {
            xcb().xcb_wait_for_special_event(
                self.as_ptr(),
                evkey.0.as_ptr(),
            )
        };

        let event = if event.is_null() {
            return Err(self.take_error().unwrap_or_else(|| Error::make_msg("Failed to wait for event")));
        } else {
            event
        };

        unsafe { self.parse_event(event) }
    }

    /// Poll for a special event.
    pub(crate) fn xcb_poll_for_special_event(&self, evkey: &SpecialEvent) -> Result<Option<Event>> {
        let event = unsafe {
            xcb().xcb_poll_for_special_event(
                self.as_ptr(),
                evkey.0.as_ptr(),
            )
        };

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
    pub(crate) fn send_request(
        &self,
        request: RawRequest,
    ) -> Result<u64> {
        let extension = request.extension().map(|ext| {
            xcb().extension(ext).unwrap_or_else(|| {
                panic!("Cannot find extension pointer for {}", ext)
            })

            // TODO: register extension in ext map
        });

        let variant = request.variant();
        let (buf, fds) = request.into_raw_parts();
        let opcode = if extension.is_some() {
            buf[1]
        } else { buf[2] };

        // build the buffers
        let mut iov = [
            empty_iov(), empty_iov(),
            Iovec {
                iov_base: buf.as_mut_ptr().cast(),
                iov_len: buf.len(),
            },
        ];

        // determine protocol request
        let proto_request = ProtocolRequest {
            count: iov.len(),
            extension: extension.unwrap_or(null_mut()),
            opcode,
            isvoid: matches!(variant, ReplyFdKind::NoReply) as u8,
        };

        let mut sr_flags = flags::CHECKED;
        let reply_has_fds = matches!(variant, ReplyFdKind::ReplyWithFDs);
        if reply_has_fds {
            sr_flags |= flags::REPLY_HAS_FDS;
        }

        let seq = if fds.is_empty() {
            // no fds
            unsafe {
                xcb().xcb_send_request64(
                    self.as_ptr(),
                    sr_flags,
                    iov.as_mut_ptr(),
                    &proto_request
                )
            }
        } else {
            // we have fds
            let fds = ManuallyDrop::new(fds.into_iter().map(|fd| fd.into_raw_fd()).collect::<Vec<_>>());
            unsafe {
                xcb().xcb_send_request_with_fds64(
                    self.as_ptr(),
                    sr_flags,
                    iov.as_mut_ptr(),
                    &proto_request,
                    fds.len() as i32,
                    fds.as_mut_ptr(),
                )
            }
        };

        // setup sequence number
        if reply_has_fds {
            self.has_fds.insert(seq);
        }

        Ok(seq)
    }

    #[cfg(unix)]
    unsafe fn extract_fds(
        &self,
        reply: &[u8],
        seq: u64,
    ) -> Vec<c_int> {
        // if the sequenc number is not in our set, return
        if self.has_fds.remove(&seq).is_none() {
            return Vec::new();
        }

        let nfds = reply[1];
        let fd_ptr = (reply.as_ptr() as *const c_int).add(reply.len());
        let fd_slice = slice::from_raw_parts(fd_ptr, nfds as usize);

        fd_slice.into()
    }

    #[cfg(not(unix))]
    unsafe fn extract_fds(
        &self,
        _reply: &[u8],
        _seq: u64
    ) -> Vec<c_int> {
        Vec::new()
    }

    unsafe fn wrap_error(&self, error: *mut GenericError) -> Error {
        use breadx::protocol::X11Error; 

        let error_ptr = error as *mut [u8; 32];
        let error_boxed = unsafe { CBox::new(error_ptr) };

        // parse it
        X11Error::try_parse(&*error_boxed, &*self.extension_manager.read())
            .map_or_else(|err| Error::from(err), |err| Error::from(err))
    }

    /// Poll for a reply.
    pub(crate) fn poll_for_reply(
        &self,
        seq: u64
    ) -> Result<Option<XcbReply>> {
        // call poll_for_reply()
        let mut reply = null_mut();
        let mut error = null_mut();
        
        // poll for it
        let found = unsafe {
            xcb().xcb_poll_for_reply64(
                self.as_ptr(),
                seq,
                &mut reply,
                &mut error,
            )
        };

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

        Ok(Some(XcbReply {
            reply,
            fds
        }))
    }

    // Wait for a reply.
    pub(crate) fn wait_for_reply(
        &self,
        seq: u64,
    ) -> Result<XcbReply> {
        // call wait_for_reply()
        let mut error = null_mut();

        let reply = unsafe {
            xcb().xcb_wait_for_reply64(
                self.as_ptr(),
                seq,
                &mut error,
            )
        };

        match (reply.is_null(), error.is_null()) {
            (true, true) => {
                // both null indicates an I/O error
                Err(self.take_maybe_error())
            }
            (false, true) => {
                // reply is non-null, return it
                let reply = unsafe { wrap_reply(reply) };
                let fds = unsafe { self.extract_fds(reply.as_ref(), seq) };

                Ok(XcbReply {
                    reply,
                    fds,
                })
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
}

impl Drop for XcbConnection {
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
    let header = unsafe {
        slice::from_raw_parts(reply as *mut u8 as *const u8, 32)
    };

    let length = u32::from_ne_bytes(
        [header[4], header[5], header[6], header[7]]
    );

    // length is 32 plus four times the len
    let length = 32usize + (4 * (length as usize));
    let reply = slice_from_raw_parts_mut(reply as *mut u8, length);

    unsafe { CBox::new(reply) }
}

pub(crate) struct SpecialEvent(CBox<EventQueueKey>);

pub(crate) struct XcbReply {
    reply: CBox<[u8]>,
    fds: Vec<c_int>,
}

impl From<XcbReply> for RawReply {
    fn from(xcr: XcbReply) -> Self {
        let XcbReply { reply, fds } = xcr;

        let data = reply.clone_slice().into_boxed_slice();
        let fds = fds.into_iter().map(|fd| {
            cfg_if::cfg_if! {
                if #[cfg(unix)] {
                    breadx::Fd::new(fd)
                } else {
                    unreachable!()
                }
            }
        }).collect::<Vec<breadx::Fd>>();

        RawReply::new(
            data,
            fds
        )
    }
}

fn auth_info(
    auth_name: &[u8],
    auth_data: &[u8],
) -> AuthInfo {
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
type HashSet<T> = hashbrown::HashSet<
    T, 
    core::hash::BuildHasherDefault<rustc_hash::FxHasher>
>;