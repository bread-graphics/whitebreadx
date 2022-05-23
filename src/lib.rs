// MIT/Apache2 License

//! Implementations of [`breadx`] connections over types from
//! `libxcb` and `libX11`.
//! 
//! Despite the advantages of pure-Rust implementations of X11
//! connections, there is one crucial disadvantage. There is a
//! backlog of 35 years worth of libraries built over `libX11`,
//! and 15 built over `libxcb`. By using a pure-Rust implementation,
//! you're preventing yourself from taking advantage of these
//! often-unimplementable libraries.
//! 
//! This crate provides wrappers over the `xcb_connection_t` type
//! from `libxcb` and the `Display` type from `libX11`. These types
//! implement the [`Display`] trait from `breadx`, allowing them to
//! be used in any library/position that supports `breadx`. Simultaneously,
//! they can be converted into pointers to their underlying representations,
//! allowing them to be used in existing `libxcb`/`libX11` libraries.
//! 
//! TODO: supported libxcb/libX11 versions
//! 
//! ## Features
//! 
//! - `real_mutex` (enabled by default) - This feature imports `std` so
//!   that all synchronous data can be locked behind standard library
//!   mutex types. With this feature disabled, the standard library is
//!   not used, but spinlocks are used to secure data instead.
//! - `xlib` (enabled by default) - Enables use of the `libX11`-based 
//!   [`Display`]s.
//! - `dl` - By default, this library statically links to `libxcb` and.
//!   optionally, `libX11`. Enabling this feature uses dynamic, runtime
//!   linking instead. This also imports the standard library.
//! - `pl` - Uses `parking_lot` mutexes instead of `std` mutexes throughout
//!   the program. Implies `real_mutex`.
//! - `to_socket` - On Unix, enables the [`XcbConnection::connect_to_socket`]
//!   function, which allows one to safely wrap around any [`AsRawFd`] type.
//!   Also imports the standard library.

#![no_std]

extern crate alloc;

#[cfg(any(
    feature = "real_mutex",
    feature = "dl"
))]
extern crate std;

#[path = "alloc.rs"]
pub(crate) mod cbox;
pub(crate) mod sync;
pub(crate) mod xcb_ffi;

mod xcb_connection;
use xcb_connection::XcbConnection;

use breadx::{Result, display::{Display, DisplayBase, RawReply, RawRequest}, protocol::{xproto::{Setup, GetInputFocusRequest}, Event}};
use cstr_core::CString;
use libc::{c_int, c_void};

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
    conn: XcbConnection,
    default_screen: usize,
}

impl XcbDisplay {
    /// Connect to the X server.
    pub fn connect(display: Option<&str>) -> Result<Self> {
        let display = display.map(|display| {
            CString::new(display).expect("CString serialization failed")
        });

        let (conn, screen) = XcbConnection::connect(display.as_deref())?;

        Ok(Self {
            conn,
            default_screen: screen as usize,
        })
    }

    /// Connect to the X server using the given authorization info.
    pub fn connect_with_auth_info(
        display: Option<&str>,
        auth_name: &[u8],
        auth_data: &[u8],
    ) -> Result<Self> {
        let display = display.map(|display| {
            CString::new(display).expect("CString serialization failed")
        });

        let (conn, screen) = XcbConnection::connect_with_auth_info(
            display.as_deref(),
            auth_name,
            auth_data
        )?;

        Ok(Self {
            conn,
            default_screen: screen as usize,
        })
    }

    /// Connect to a file descriptor.
    /// 
    /// # Safety
    /// 
    /// The FD must be a valid socket.
    pub unsafe fn connect_to_fd(
        fd: c_int,
        screen: usize,
        auth_name: &[u8],
        auth_data: &[u8],
    ) -> Result<Self> {
        Ok(Self {
            conn: XcbConnection::connect_to_fd(fd, auth_name, auth_data)?,
            default_screen: screen,
        })
    }

    /// Create a new `XcbConnection` as a wrapper around an existing
    /// pointer to an `xcb_connection_t`.
    /// 
    /// The `disconnect` parameter tells whether we logically own the
    /// connection, and if we should disconnect it ourselves when the
    /// structure is dropped.
    /// 
    /// ## Safety
    /// 
    /// `ptr` must be a valid, non-null pointer to a `libxcb` connection.
    /// Behavior is undefined if any of these variants do not hold.
    pub unsafe fn from_ptr(
        ptr: *mut c_void,
        disconnect: bool,
        screen: usize,
    ) -> Self {
        let conn = XcbConnection::from_ptr(
            ptr.cast(),
            disconnect,
        );

        Self { conn, default_screen: screen }
    }

    /// Get the pointer to the underlying `libxcb` connection.
    /// 
    /// # Safety
    /// 
    /// It is impossible to use this pointer for anything unsafe without
    /// other unsafe code. Here are some notes for using it in unsafe code:
    /// 
    /// - It must not be disconnected, unless this structure was created
    ///   through [`from_ptr`] with `false` for the `disconnect` parameter.
    /// - Requests sent on the `libxcb` end shouldn't be received on the
    ///   `breadx` end, and vice versa.
    pub fn as_ptr(&self) -> *mut c_void {
        self.conn.as_ptr().cast()
    }

    /// Get the underlying file descriptor for this connection.
    pub fn file_descriptor(&self) -> c_int {
        self.conn.get_fd()
    }

    fn synchronize_impl(&self) -> Result<()> {
        // similar to the breadx synchronize
        let req = GetInputFocusRequest {};
        let req = RawRequest::from_request_reply(req);
        let sequence = self.send_request_raw(req)?;

        // wait for it
        self.flush()?;
        let _ = self.wait_for_reply_raw(sequence)?;
        Ok(())
    }
}

impl DisplayBase for XcbDisplay {
    fn setup(&self) -> &Setup {
        self.conn.get_setup()
    }

    fn default_screen_index(&self) -> usize {
        self.default_screen
    }

    fn poll_for_event(&mut self) -> Result<Option<Event>> {
        self.conn.poll_for_event()
    }

    fn poll_for_reply_raw(&mut self, seq: u64) -> Result<Option<RawReply>> {
        self.conn.poll_for_reply(seq).map(|o| o.map(Into::into))
    }
}

impl DisplayBase for &XcbDisplay {
    fn setup(&self) -> &Setup {
        self.conn.get_setup()
    }

    fn default_screen_index(&self) -> usize {
        self.default_screen
    }

    fn poll_for_event(&mut self) -> Result<Option<Event>> {
        self.conn.poll_for_event()
    }

    fn poll_for_reply_raw(&mut self, seq: u64) -> Result<Option<RawReply>> {
        self.conn.poll_for_reply(seq).map(|o| o.map(Into::into))
    }
}

impl Display for XcbDisplay {
    fn send_request_raw(&mut self, req: RawRequest) -> Result<u64> {
        self.conn.send_request(req)   
    }

    fn wait_for_reply_raw(&mut self, seq: u64) -> Result<RawReply> {
        self.conn.wait_for_reply(seq).map(Into::into)
    }

    fn flush(&mut self) -> Result<()> {
        self.conn.flush()
    }

    fn wait_for_event(&mut self) -> Result<Event> {
        self.conn.wait_for_event()
    }

    fn generate_xid(&mut self) -> Result<u32> {
        self.conn.generate_xid()
    }

    fn maximum_request_length(&mut self) -> Result<usize> {
        self.conn.maximum_request_length()
    }

    fn synchronize(&mut self) -> Result<()> {
        self.synchronize_impl()
    }
}

impl Display for &XcbDisplay {
    fn send_request_raw(&mut self, req: RawRequest) -> Result<u64> {
        self.conn.send_request(req)   
    }

    fn wait_for_reply_raw(&mut self, seq: u64) -> Result<RawReply> {
        self.conn.wait_for_reply(seq).map(Into::into)
    }

    fn flush(&mut self) -> Result<()> {
        self.conn.flush()
    }

    fn wait_for_event(&mut self) -> Result<Event> {
        self.conn.wait_for_event()
    }

    fn generate_xid(&mut self) -> Result<u32> {
        self.conn.generate_xid()
    }

    fn maximum_request_length(&mut self) -> Result<usize> {
        self.conn.maximum_request_length()
    }

    fn synchronize(&mut self) -> Result<()> {
        self.synchronize_impl()
    }
}