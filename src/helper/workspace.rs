use std::{ffi, ptr};

pub(crate) type Initializer = unsafe extern "C" fn(
    arg_in: *mut *mut ffi::c_void,
    arg_out: *mut *mut ffi::c_void,
    arg_val: *mut *mut ffi::c_void,
);
pub(crate) type Finalizer = unsafe extern "C" fn(
    arg_in: *mut ffi::c_void,
    arg_out: *mut ffi::c_void,
    arg_val: *mut ffi::c_void,
);

pub(crate) struct Workspace {
    pub(crate) input_ptr: *mut ffi::c_void, // const after initialization
    pub(crate) output_ptr: *mut ffi::c_void,
    pub(crate) validation_ptr: Option<*mut ffi::c_void>, // const after initialization
}

impl Default for Workspace {
    fn default() -> Self {
        Workspace {
            input_ptr: ptr::null_mut(),
            output_ptr: ptr::null_mut(),
            validation_ptr: None,
        }
    }
}

unsafe impl Sync for Workspace {}
