pub(crate) mod hook;
pub(crate) mod workspace;

use std::ffi;

pub(crate) type Evaluator =
    unsafe extern "C" fn(arg_in: *mut ffi::c_void, arg_out: *mut ffi::c_void) -> f64;
pub(crate) type Validator =
    unsafe extern "C" fn(arg_val: *const ffi::c_void, arg_out: *const ffi::c_void) -> bool;
