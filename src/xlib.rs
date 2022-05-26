// MIT/Apache2 License

use crate::{
    sync::{call_once, OnceCell},
    xlib_ffi::{xlib, X11Ffi, XDisplay},
    XcbDisplay,
};
use __private::Sealed;
use breadx::{
    display::{Display, DisplayBase, RawReply, RawRequest},
    protocol::{xproto::Setup, Event},
    Error, Result,
};
use core::{
    cell::Cell,
    marker::PhantomData,
    ptr::{null, NonNull},
};
use cstr_core::CStr;
use libc::c_void;

#[cfg(all(unix, feature = "to_socket"))]
use std::os::unix::io::{AsRawFd, RawFd};

/// A display that acts as a wrapper around a `libX11` display.
pub struct XlibDisplay<ThreadSafety> {
    xcb: XcbDisplay,
    display: NonNull<XDisplay>,
    disconnect: bool,
    _phantom: PhantomData<ThreadSafety>,
}

/// Represents a type that can define the thread-safety for the `XlibDisplay`.'
pub trait ThreadSafety: Sealed {
    /// If a function needs to be called to initialize this variant,
    /// call it here.
    fn initialize() -> Result<()>;
}

/// The display is not thread safe.
///
/// It is able to be shared between threads, but cannot be used
/// concurrently.
pub struct ThreadUnsafe {
    _private: PhantomData<Cell<()>>,
}

impl ThreadSafety for ThreadUnsafe {
    fn initialize() -> Result<()> {
        Ok(())
    }
}

/// The display is completely thread safe.
pub struct ThreadSafe {
    _private: (),
}

impl ThreadSafety for ThreadSafe {
    fn initialize() -> Result<()> {
        static THREADS_INIT: OnceCell<libc::c_int> = OnceCell::new();

        let result = call_once(&THREADS_INIT, || {
            // call XInitThreads to initialize the threading system
            unsafe { xlib().XInitThreads() }
        });

        match *result {
            0 => Err(Error::make_msg("failed to initialize threading")),
            _ => Ok(()),
        }
    }
}

impl<TS: ThreadSafety> XlibDisplay<TS> {
    /// Connect to the server using the given `display_name`.
    pub fn connect(name: Option<&CStr>) -> Result<Self> {
        // initialize thread safety if applicable
        TS::initialize()?;

        let display_name = name.map_or(null(), |name| name.as_ptr());

        // connect!
        let conn = unsafe { xlib().XOpenDisplay(display_name) };

        // check for null
        if conn.is_null() {
            return Err(Error::make_msg("failed to connect to X server"));
        }

        Ok(unsafe { Self::from_ptr(conn.cast(), true) })
    }

    /// Create a new `XlibDisplay` from an existing pointer to an
    /// X11 `Display`.
    ///
    /// # Safety
    ///
    /// The pointer must be a valid, non-null pointer to an X11 `Display`.
    pub unsafe fn from_ptr(ptr: *mut c_void, disconnect: bool) -> Self {
        let conn: *mut XDisplay = ptr.cast();

        // get the default screen, needed for XcbDisplay innards
        let screen = unsafe { xlib().XDefaultScreen(conn) };

        // get the internal XCB connection
        let xcb_conn = unsafe { xlib().XGetXCBConnection(conn) };

        // create the XcbDisplay
        let xcb = unsafe { XcbDisplay::from_ptr(xcb_conn.cast(), false, screen as usize) };

        // we're live
        Self {
            xcb,
            display: NonNull::new_unchecked(conn),
            disconnect,
            _phantom: PhantomData,
        }
    }
}

impl<TS> XlibDisplay<TS> {
    /// Get the interior `libX11` `Display` that backs this connection.
    pub fn as_xlib_connection(&self) -> *mut c_void {
        self.display.as_ptr().cast()
    }

    /// Get the interior `libxcb` `Connection` that backs this connection.
    pub fn as_xcb_connection(&self) -> *mut c_void {
        self.xcb.as_raw_connection()
    }
}

#[cfg(all(unix, feature = "to_socket"))]
impl<TS> AsRawFd for XlibDisplay<TS> {
    fn as_raw_fd(&self) -> RawFd {
        self.xcb.as_raw_fd()
    }
}

impl<TS> DisplayBase for XlibDisplay<TS> {
    fn setup(&self) -> &Setup {
        self.xcb.setup()
    }

    fn default_screen_index(&self) -> usize {
        self.xcb.default_screen_index()
    }

    fn poll_for_event(&mut self) -> Result<Option<Event>> {
        self.xcb.poll_for_event()
    }

    fn poll_for_reply_raw(&mut self, seq: u64) -> Result<Option<RawReply>> {
        self.xcb.poll_for_reply_raw(seq)
    }
}

impl<TS> DisplayBase for &XlibDisplay<TS> {
    fn setup(&self) -> &Setup {
        self.xcb.setup()
    }

    fn default_screen_index(&self) -> usize {
        self.xcb.default_screen_index()
    }

    fn poll_for_event(&mut self) -> Result<Option<Event>> {
        (&self.xcb).poll_for_event()
    }

    fn poll_for_reply_raw(&mut self, seq: u64) -> Result<Option<RawReply>> {
        (&self.xcb).poll_for_reply_raw(seq)
    }
}

impl<TS> Display for XlibDisplay<TS> {
    fn flush(&mut self) -> Result<()> {
        self.xcb.flush()
    }

    fn generate_xid(&mut self) -> Result<u32> {
        self.xcb.generate_xid()
    }

    fn maximum_request_length(&mut self) -> Result<usize> {
        self.xcb.maximum_request_length()
    }

    fn send_request_raw(&mut self, req: RawRequest<'_, '_>) -> Result<u64> {
        self.xcb.send_request_raw(req)
    }

    fn synchronize(&mut self) -> Result<()> {
        self.xcb.synchronize()
    }

    fn wait_for_event(&mut self) -> Result<Event> {
        self.xcb.wait_for_event()
    }

    fn wait_for_reply_raw(&mut self, seq: u64) -> Result<RawReply> {
        self.xcb.wait_for_reply_raw(seq)
    }
}

impl<TS> Display for &XlibDisplay<TS> {
    fn flush(&mut self) -> Result<()> {
        (&self.xcb).flush()
    }

    fn generate_xid(&mut self) -> Result<u32> {
        (&self.xcb).generate_xid()
    }

    fn maximum_request_length(&mut self) -> Result<usize> {
        (&self.xcb).maximum_request_length()
    }

    fn send_request_raw(&mut self, req: RawRequest<'_, '_>) -> Result<u64> {
        (&self.xcb).send_request_raw(req)
    }

    fn synchronize(&mut self) -> Result<()> {
        (&self.xcb).synchronize()
    }

    fn wait_for_event(&mut self) -> Result<Event> {
        (&self.xcb).wait_for_event()
    }

    fn wait_for_reply_raw(&mut self, seq: u64) -> Result<RawReply> {
        (&self.xcb).wait_for_reply_raw(seq)
    }
}

impl<TS> Drop for XlibDisplay<TS> {
    fn drop(&mut self) {
        if self.disconnect {
            unsafe {
                xlib().XCloseDisplay(self.display.as_ptr());
            }
        }
    }
}

mod __private {
    pub trait Sealed {
        fn __sealed_trait_marker() {}
    }

    impl Sealed for super::ThreadUnsafe {}
    impl Sealed for super::ThreadSafe {}
}
