use std::ptr;

pub(crate) trait OrNull<T> {
    fn or_null(self) -> *mut T;
}

impl<T> OrNull<T> for Option<*mut T> {
    fn or_null(self) -> *mut T {
        match self {
            Some(ptr) => ptr,
            None => ptr::null_mut(),
        }
    }
}
