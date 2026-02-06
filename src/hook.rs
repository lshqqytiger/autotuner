use crate::{
    context::{self, Context},
    parameter::{IntegerSpace, Specification, Value},
    utils::interner::Intern,
    workspace::Workspace,
};
use libloading::Symbol;
use serde::{Deserialize, Serialize};
use std::{ffi, ptr};

#[derive(Serialize, Deserialize)]
pub(crate) struct Configuration {
    pub(crate) pre: Vec<String>,
    pub(crate) post: Vec<String>,
}

type Function = unsafe extern "C" fn(
    ctx: *mut Context,
    ws: *const Workspace,
    get: extern "C" fn(id: ffi::c_int) -> *const ffi::c_void,
);

pub(crate) struct Hook<'a>(Symbol<'a, Function>);

impl<'a> From<Symbol<'a, Function>> for Hook<'a> {
    fn from(f: Symbol<'a, Function>) -> Self {
        Hook(f)
    }
}

impl<'a> Hook<'a> {
    pub(crate) fn call(&self, context: &mut Context, workspace: &Workspace) {
        unsafe {
            self.0(context as _, workspace as _, get as _);
        }
    }
}

#[repr(u32)]
enum Interface {
    TempDir = 0x00,

    ParameterGetInt = 0x10,

    WorkspaceGetPtr = 0x20,

    AppendArgument = 0x30,

    Invalidate = 0x40,
}

impl TryFrom<ffi::c_int> for Interface {
    type Error = ();

    fn try_from(value: ffi::c_int) -> Result<Self, Self::Error> {
        match value {
            x if x == Interface::TempDir as ffi::c_int => Ok(Interface::TempDir),
            x if x == Interface::ParameterGetInt as ffi::c_int => Ok(Interface::ParameterGetInt),
            x if x == Interface::WorkspaceGetPtr as ffi::c_int => Ok(Interface::WorkspaceGetPtr),
            x if x == Interface::AppendArgument as ffi::c_int => Ok(Interface::AppendArgument),
            x if x == Interface::Invalidate as ffi::c_int => Ok(Interface::Invalidate),
            _ => Err(()),
        }
    }
}

extern "C" fn get(id: ffi::c_int) -> *const ffi::c_void {
    match Interface::try_from(id) {
        Ok(Interface::TempDir) => temp_dir as *const ffi::c_void,
        Ok(Interface::ParameterGetInt) => parameter_get_int as *const ffi::c_void,
        Ok(Interface::WorkspaceGetPtr) => workspace_get_ptr as *const ffi::c_void,
        Ok(Interface::AppendArgument) => append_argument as *const ffi::c_void,
        Ok(Interface::Invalidate) => invalidate as *const ffi::c_void,
        _ => ptr::null(),
    }
}

extern "C" fn temp_dir(ctx: *mut Context, ptr: *mut ffi::c_char, size: usize) {
    let ctx = if let Some(ctx) = unsafe { ctx.as_ref() } {
        ctx
    } else {
        return;
    };
    let len = ctx.temp_dir.len().min(size - 1);
    unsafe {
        ptr.copy_from_nonoverlapping(ctx.temp_dir.as_ptr() as _, len);
        *ptr.add(len) = 0;
    }
}

extern "C" fn parameter_get_int(ctx: *mut Context, name: *const ffi::c_char) -> *const ffi::c_int {
    let ctx = if let Some(ctx) = unsafe { ctx.as_ref() } {
        ctx
    } else {
        return ptr::null();
    };
    let name = unsafe { ffi::CStr::from_ptr(name) }
        .to_string_lossy()
        .into_owned()
        .intern();
    let specification: &Specification = if let Some(specification) = ctx.profile.0.get(&name) {
        specification
    } else {
        return ptr::null();
    };
    let value = if let Some(value) = ctx.instance.parameters.get(&name) {
        value
    } else {
        return ptr::null();
    };
    match (specification, value) {
        (
            Specification::Integer {
                transformer: _,
                space: IntegerSpace::Sequence(_, _, _),
            },
            Value::Integer(v),
        ) => v as *const ffi::c_int,
        (
            Specification::Integer {
                transformer: _,
                space: IntegerSpace::Candidates(candidates, _),
            },
            Value::Index(i),
        ) => &candidates[*i] as *const ffi::c_int,
        _ => ptr::null(),
    }
}

extern "C" fn workspace_get_ptr(
    ws: *const Workspace,
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

extern "C" fn append_argument(ctx: *mut Context, argument: *const ffi::c_char) {
    let ctx = if let Some(ctx) = unsafe { ctx.as_mut() } {
        ctx
    } else {
        return;
    };
    ctx.arguments.push(
        unsafe { ffi::CStr::from_ptr(argument) }
            .to_string_lossy()
            .into_owned(),
    );
}

extern "C" fn invalidate(ctx: *mut Context) {
    let ctx = if let Some(ctx) = unsafe { ctx.as_mut() } {
        ctx
    } else {
        return;
    };
    ctx.result = context::Result::Invalid;
}
