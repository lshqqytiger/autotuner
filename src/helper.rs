use crate::workspace::Workspace;
use libloading::Symbol;
use serde::{Deserialize, Serialize};
use std::{ffi, ptr};

#[derive(Serialize, Deserialize)]
pub(crate) struct Configuration {
    pub(crate) pre: String,
    pub(crate) post: String,
}

type Function = unsafe extern "C" fn(
    ws: *mut Workspace,
    get: extern "C" fn(id: ffi::c_int) -> *const ffi::c_void,
);

pub(crate) struct Helper<'a>(Symbol<'a, Function>);

impl<'a> From<Symbol<'a, Function>> for Helper<'a> {
    fn from(f: Symbol<'a, Function>) -> Self {
        Helper(f)
    }
}

impl<'a> Helper<'a> {
    pub(crate) fn call(&self, workspace: &mut Workspace) {
        unsafe {
            self.0(workspace as _, get as _);
        }
    }
}

#[repr(u32)]
enum Interface {
    WorkspaceSetPtr = 0x00,

    WorkspaceGetPtr = 0x10,
}

impl TryFrom<ffi::c_int> for Interface {
    type Error = ();

    fn try_from(value: ffi::c_int) -> Result<Self, Self::Error> {
        match value {
            x if x == Interface::WorkspaceSetPtr as ffi::c_int => Ok(Interface::WorkspaceSetPtr),
            x if x == Interface::WorkspaceGetPtr as ffi::c_int => Ok(Interface::WorkspaceGetPtr),
            _ => Err(()),
        }
    }
}

extern "C" fn get(id: ffi::c_int) -> *const ffi::c_void {
    match Interface::try_from(id) {
        Ok(Interface::WorkspaceSetPtr) => workspace_set_ptr as *const ffi::c_void,
        Ok(Interface::WorkspaceGetPtr) => workspace_get_ptr as *const ffi::c_void,
        _ => ptr::null(),
    }
}

extern "C" fn workspace_set_ptr(
    ws: *mut Workspace,
    name: *const ffi::c_char,
    ptr: *mut ffi::c_void,
) {
    let ws = if let Some(ws) = unsafe { ws.as_mut() } {
        ws
    } else {
        return;
    };
    let name = if let Some(name) = unsafe { ffi::CStr::from_ptr(name).to_str().ok() } {
        name
    } else {
        return;
    };
    ws.0.insert(name, ptr);
}

extern "C" fn workspace_get_ptr(
    ws: *mut Workspace,
    name: *const ffi::c_char,
) -> *const *mut ffi::c_void {
    let ws = if let Some(ws) = unsafe { ws.as_ref() } {
        ws
    } else {
        return ptr::null();
    };
    let name = if let Some(name) = unsafe { ffi::CStr::from_ptr(name).to_str().ok() } {
        name
    } else {
        return ptr::null();
    };
    if let Some(ptr) = ws.0.get(name) {
        ptr
    } else {
        ptr::null()
    }
}
