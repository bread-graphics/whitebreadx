//               Copyright John Nunley, 2022.
// Distributed under the Boost Software License, Version 1.0.
//       (See accompanying file LICENSE or copy at
//         https://www.boost.org/LICENSE_1_0.txt)

//! Provides a simple wrapper over C allocations.

use alloc::vec::Vec;
use core::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

/// An allocation, made using the `libc` `alloc` function.
pub(crate) struct CBox<T: ?Sized> {
    ptr: NonNull<T>,
}

impl<T: ?Sized> CBox<T> {
    /// Creates a new `CBox` from a pointer.
    ///
    /// # Safety
    ///
    /// The pointer must be valid, not null and
    /// made from `alloc`.
    pub(crate) unsafe fn new(ptr: *mut T) -> Self {
        CBox {
            ptr: NonNull::new_unchecked(ptr),
        }
    }

    /// Returns the pointer.
    pub(crate) fn as_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Returns the inner data as a reference.
    pub(crate) fn as_ref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    /// Returns the inner data as a mutable reference.
    pub(crate) fn as_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T: ?Sized> Deref for CBox<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<T: ?Sized> DerefMut for CBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}

impl<T: Clone> CBox<[T]> {
    /// Clone the allocation's data.
    pub(crate) fn clone_slice(&self) -> Vec<T> {
        self.as_ref().into()
    }
}

impl<T: ?Sized> Drop for CBox<T> {
    fn drop(&mut self) {
        unsafe {
            libc::free(self.as_ptr() as *mut libc::c_void);
        }
    }
}
