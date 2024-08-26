pub struct AliasBox<T> {
    ptr: *const T,
}

impl<T> AliasBox<T> {
    pub fn new(value: T) -> Self {
        Self {
            ptr: Box::into_raw(Box::new(value)),
        }
    }

    pub fn as_ptr(&self) -> *mut T {
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
