//               Copyright John Nunley, 2022.
// Distributed under the Boost Software License, Version 1.0.
//       (See accompanying file LICENSE or copy at
//         https://www.boost.org/LICENSE_1_0.txt)


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
//! ## External Library Version Support
//! 
//! The minimum supported versions of `libxcb` and `libX11` necessary for 
//! this library are unknown. This library has been tested to work with 
//! `libxcb` version 1.14 and `libX11` version 2:1.7. However, the `libX11` 
//! version must be after the paradigm shift where it began using `libxcb` 
//! as an internal transport. There are no plans to support legacy `libX11`.
//!
//! ## Features
//!
//! - `real_mutex` (enabled by default) - This feature imports `std` so
//!   that all synchronous data can be locked behind standard library
//!   mutex types. With this feature disabled, the standard library is
//!   not used, but spinlocks are used to secure data instead. Think
//!   carefully before disabling this feature, since spinlocks are
//!   [considered harmful].
//! - `xlib` (enabled by default) - Enables use of the `libX11`-based
//!   [`Display`]s.
//! - `dl` - By default, this library statically links to `libxcb` and.
//!   optionally, `libX11`. Enabling this feature uses dynamic, runtime
//!   linking instead. This also imports the standard library.
//! - `pl` - Uses `parking_lot` mutexes instead of `std` mutexes throughout
//!   the program. Implies `real_mutex`.
//! - `to_socket` - On Unix, enables the [`XcbDisplay::connect_to_socket`]
//!   function, which allows one to safely wrap around any [`AsRawFd`] type.
//!   Also imports the standard library and adds `AsRawFd` impls to
//!   `XcbDisplay` and `XlibDisplay`.
//! 
//! [considered harmful]: https://matklad.github.io/2020/01/02/spinlocks-considered-harmful.html

#![no_std]
#![allow(unused_unsafe)]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[path = "alloc.rs"]
pub(crate) mod cbox;
pub(crate) mod extension_manager;
pub(crate) mod sync;
pub(crate) mod xcb_ffi;

#[cfg(feature = "xlib")]
pub(crate) mod xlib_ffi;

mod xcb_connection;
pub use xcb_connection::XcbDisplay;

#[cfg(feature = "xlib")]
mod xlib;
#[cfg(feature = "xlib")]
pub use xlib::{ThreadSafe, ThreadSafety, ThreadUnsafe, XlibDisplay};
