use crate::{helper::Initializer, metadata::Metadata};
use libloading::{Library, Symbol};
use std::{ffi, ptr};

// TODO: input_ptr and validation_ptr can be shared between threads
pub(crate) struct Workspace {
    pub(crate) input_ptr: *mut ffi::c_void, // const after initialization
    pub(crate) output_ptr: *mut ffi::c_void,
    pub(crate) validation_ptr: Option<*mut ffi::c_void>, // const after initialization
}

impl Workspace {
    pub(crate) fn new(lib: &Library, metadata: &Metadata) -> Result<Self, libloading::Error> {
        let initializer: Symbol<Initializer> = unsafe { lib.get(metadata.initializer.as_bytes()) }?;
        let mut input_ptr = ptr::null_mut();
        let mut output_ptr = ptr::null_mut();
        let mut validation_ptr = if metadata.validator.is_some() {
            Some(ptr::null_mut())
        } else {
            None
        };

        unsafe {
            initializer(
                &mut input_ptr,
                &mut output_ptr,
                if let Some(ptr) = validation_ptr.as_mut() {
                    ptr
                } else {
                    ptr::null_mut()
                },
            );
        }

        Ok(Workspace {
            input_ptr,
            output_ptr,
            validation_ptr,
        })
    }
}

unsafe impl Sync for Workspace {}
