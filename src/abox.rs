//! Implementation of a `Box` for self referential types.
//! 
//! Reference: https://morestina.net/blog/1868/self-referential-types-for-fun-and-profit


/// Aliasable implementation of `Box`.
/// 
/// Provides the same semantics as a Box, but prevents the optimizer from assuming that moving it
/// invalidates references to its contents
pub struct AliasBox<T> {
    ptr: *const T,
}

impl<T> AliasBox<T> {
    /// Allocates memory on the heap and then places `x` into it.
    pub fn new(x: T) -> Self {
        Self {
            ptr: Box::into_raw(Box::new(x)),
        }
    }

    /// Obtains the inner pointer as mutable.
    fn as_ptr(&self) -> *mut T {
        self.ptr as *mut T
    }
}

impl<T> std::ops::Deref for AliasBox<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.ptr }
    }
}

impl<T> std::ops::DerefMut for AliasBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.as_ptr() }
    }
}

impl<T> Drop for AliasBox<T> {
    fn drop(&mut self) {
        unsafe {
            drop(Box::from_raw(self.ptr as *mut T));
        }
    }
}
