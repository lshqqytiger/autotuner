use crate::ffi::context::Context;
use libloading::Symbol;
use std::{ffi, ptr};

type Function = unsafe extern "C" fn(
    ctx: *mut Context,
    get: extern "C" fn(id: ffi::c_int) -> *const ffi::c_void,
);

pub(crate) struct Runner<'a>(Symbol<'a, Function>);

impl<'a> From<Symbol<'a, Function>> for Runner<'a> {
    fn from(f: Symbol<'a, Function>) -> Self {
        Runner(f)
    }
}

impl<'a> Runner<'a> {
    pub(crate) fn call(&self, context: &mut Context) {
        unsafe {
            self.0(context as _, get as _);
        }
    }
}

#[repr(u32)]
enum Interface {
    GetPtr = 0x00,

    SetResult = 0x10,
}

impl TryFrom<ffi::c_int> for Interface {
    type Error = ();

    fn try_from(value: ffi::c_int) -> Result<Self, Self::Error> {
        match value {
            x if x == Interface::GetPtr as ffi::c_int => Ok(Interface::GetPtr),
            x if x == Interface::SetResult as ffi::c_int => Ok(Interface::SetResult),
            _ => Err(()),
        }
    }
}

extern "C" fn get(id: ffi::c_int) -> *const ffi::c_void {
    match Interface::try_from(id) {
        Ok(Interface::GetPtr) => get_ptr as *const ffi::c_void,
        Ok(Interface::SetResult) => set_result as *const ffi::c_void,
        _ => ptr::null(),
    }
}

extern "C" fn get_ptr(ctx: *mut Context, name: *const ffi::c_char) -> *const *mut ffi::c_void {
    let ctx = if let Some(ctx) = unsafe { ctx.as_ref() } {
        ctx
    } else {
        return ptr::null();
    };
    let name = if let Some(name) = unsafe { ffi::CStr::from_ptr(name).to_str().ok() } {
        name
    } else {
        return ptr::null();
    };
    if let Some(ptr) = ctx.inner.workspace.0.get(name) {
        ptr
    } else {
        ptr::null()
    }
}

extern "C" fn set_result(ctx: *mut Context, result: f64) {
    let ctx = if let Some(ctx) = unsafe { ctx.as_mut() } {
        ctx
    } else {
        return;
    };
    ctx.individual.fitness = crate::individual::Fitness::Valid(result);
}
