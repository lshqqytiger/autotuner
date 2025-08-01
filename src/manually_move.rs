use std::ops::{Deref, DerefMut};

#[derive(Clone)]
#[repr(transparent)]
pub struct ManuallyMove<T: Sized>(*mut T);

impl<T: Sized> ManuallyMove<T> {
    pub fn new(value: T) -> Self {
        ManuallyMove(Box::into_raw(Box::new(value)))
    }

    pub unsafe fn mov(&self) -> ManuallyMove<T> {
        ManuallyMove(self.0)
    }
}

impl<T> Deref for ManuallyMove<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }.unwrap()
    }
}

impl<T> DerefMut for ManuallyMove<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }.unwrap()
    }
}

impl<T> Drop for ManuallyMove<T> {
    fn drop(&mut self) {
        drop(unsafe { Box::from_raw(self.0) });
    }
}

unsafe impl<T> Send for ManuallyMove<T> {}

unsafe impl<T> Sync for ManuallyMove<T> {}
