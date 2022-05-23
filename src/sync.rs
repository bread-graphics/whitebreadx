// MIT/Apache2 License

//! Current synchronization primitives for this crate.

cfg_if::cfg_if! {
    if #[cfg(not(feature = "real_mutex"))] {
        pub use spin::{Mutex, RwLock, OnceCell};
    } else {
        pub use std::sync::{Mutex, RwLock};
        pub use once_cell::sync::OnceCell;
    }
}