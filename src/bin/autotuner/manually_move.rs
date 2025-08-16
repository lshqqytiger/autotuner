use std::ops::{Deref, DerefMut};

#[repr(transparent)]
pub(crate) struct ManuallyMove<T: Sized>(*mut T);

impl<T: Sized> ManuallyMove<T> {
    pub(crate) fn new(value: T) -> Self {
        ManuallyMove(Box::into_raw(Box::new(value)))
    }

    pub(crate) unsafe fn mov(&self) -> ManuallyMove<T> {
        ManuallyMove(self.0)
    }

    pub(crate) fn drop(self) {
        drop(unsafe { Box::from_raw(self.0) });
    }
}

impl<T> Clone for ManuallyMove<T> {
    fn clone(&self) -> Self {
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

unsafe impl<T> Send for ManuallyMove<T> {}

unsafe impl<T> Sync for ManuallyMove<T> {}
