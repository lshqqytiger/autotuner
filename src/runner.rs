use crate::{context::Context, workspace::Workspace};
use libloading::Symbol;
use std::{ffi, ptr};

type Function = unsafe extern "C" fn(
    ctx: *mut Context,
    ws: *const Workspace,
    get: extern "C" fn(id: ffi::c_int) -> *const ffi::c_void,
);

pub(crate) struct Runner<'a>(Symbol<'a, Function>);

impl<'a> From<Symbol<'a, Function>> for Runner<'a> {
    fn from(f: Symbol<'a, Function>) -> Self {
        Runner(f)
    }
}

impl<'a> Runner<'a> {
    pub(crate) fn call(&self, context: &mut Context, workspace: &Workspace) {
        unsafe {
            self.0(context as _, workspace as _, get as _);
        }
    }
}

#[repr(u32)]
enum Interface {
    GetPtr = 0x00,
    Result = 0x10,
}

impl TryFrom<ffi::c_int> for Interface {
    type Error = ();

    fn try_from(value: ffi::c_int) -> Result<Self, Self::Error> {
        match value {
            x if x == Interface::GetPtr as ffi::c_int => Ok(Interface::GetPtr),
            x if x == Interface::Result as ffi::c_int => Ok(Interface::Result),
            _ => Err(()),
        }
    }
}

extern "C" fn get(id: ffi::c_int) -> *const ffi::c_void {
    match Interface::try_from(id) {
        Ok(Interface::GetPtr) => get_ptr as *const ffi::c_void,
        Ok(Interface::Result) => result as *const ffi::c_void,
        _ => ptr::null(),
    }
}

extern "C" fn get_ptr(ws: *const Workspace, name: *const ffi::c_char) -> *mut ffi::c_void {
    let ws = if let Some(ws) = unsafe { ws.as_ref() } {
        ws
    } else {
        return ptr::null_mut();
    };
    let name = if let Some(name) = unsafe { ffi::CStr::from_ptr(name).to_str().ok() } {
        name
    } else {
        return ptr::null_mut();
    };
    if let Some(ptr) = ws.0.get(name) {
        *ptr
    } else {
        ptr::null_mut()
    }
}

extern "C" fn result(context: *mut Context, result: f64) {
    let ctx = if let Some(ctx) = unsafe { context.as_mut() } {
        ctx
    } else {
        return;
    };
    ctx.result = crate::context::Result::Valid(result);
}
