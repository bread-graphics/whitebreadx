//               Copyright John Nunley, 2022.
// Distributed under the Boost Software License, Version 1.0.
//       (See accompanying file LICENSE or copy at
//         https://www.boost.org/LICENSE_1_0.txt)

//! Current synchronization primitives for this crate.

cfg_if::cfg_if! {
    if #[cfg(not(feature = "real_mutex"))] {
        pub(crate) use spin::{
            Mutex,
            MutexGuard,
            RwLock,
            RwLockReadGuard,
            RwLockWriteGuard,
            Once as OnceCell,
            lazy::Lazy,
        };

        pub(crate) fn mtx_lock<T>(mtx: &Mutex<T>) -> MutexGuard<'_, T> {
            mtx.lock()
        }

        pub(crate) fn rwl_read<T>(rwl: &RwLock<T>) -> RwLockReadGuard<'_, T> {
            rwl.read()
        }

        pub(crate) fn rwl_write<T>(rwl: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
            rwl.write()
        }

        pub(crate) fn call_once<T>(
            once: &OnceCell<T>,
            f: impl FnOnce() -> T,
        ) -> &T {
            once.call_once(move || {
                f()
            })
        }
    } else if #[cfg(all(feature = "real_mutex", not(feature = "pl")))]{
        pub(crate) use std::sync::{
            Mutex,
            MutexGuard,
            RwLock,
            RwLockReadGuard,
            RwLockWriteGuard,
        };
        pub(crate) use once_cell::sync::{OnceCell, Lazy};

        pub(crate) fn mtx_lock<T>(mtx: &Mutex<T>) -> MutexGuard<'_, T> {
            match mtx.lock() {
                Ok(guard) => guard,
                Err(poison) => poison.into_inner(),
            }
        }

        pub(crate) fn rwl_read<T>(rwl: &RwLock<T>) -> RwLockReadGuard<'_, T> {
            match rwl.read() {
                Ok(guard) => guard,
                Err(poison) => poison.into_inner(),
            }
        }

        pub(crate) fn rwl_write<T>(rwl: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
            match rwl.write() {
                Ok(guard) => guard,
                Err(poison) => poison.into_inner(),
            }
        }

        pub(crate) fn call_once<T>(
            once: &OnceCell<T>,
            f: impl FnOnce() -> T,
        ) -> &T {
            once.get_or_init(move || {
                f()
            })
        }
    } else {
         pub(crate) use parking_lot::{
            Mutex,
            MutexGuard,
            RwLock,
            RwLockReadGuard,
            RwLockWriteGuard,
        };
        pub(crate) use once_cell::sync::{OnceCell, Lazy};

        pub(crate) fn mtx_lock<T>(mtx: &Mutex<T>) -> MutexGuard<'_, T> {
            mtx.lock()
        }

        pub(crate) fn rwl_read<T>(rwl: &RwLock<T>) -> RwLockReadGuard<'_, T> {
            rwl.read()
        }

        pub(crate) fn rwl_write<T>(rwl: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
            rwl.write()
        }

        pub(crate) fn call_once<T>(
            once: &OnceCell<T>,
            f: impl FnOnce() -> T,
        ) -> &T {
            once.get_or_init(move || {
                f()
            })
        }
    }
}
