// MIT/Apache2 License

//! Current synchronization primitives for this crate.

cfg_if::cfg_if! {
    if #[cfg(not(feature = "real_mutex"))] {
        pub(crate) use spin::{
            Mutex,
            MutexGuard,
            RwLock,
            RwLockReadGuard,
            RwLockWriteGuard,
            OnceCell
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
    } else {
        pub(crate) use std::sync::{
            Mutex,
            MutexGuard,
            RwLock,
            RwLockReadGuard,
            RwLockWriteGuard,
        };
        pub(crate) use once_cell::sync::OnceCell;

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
    }
}
