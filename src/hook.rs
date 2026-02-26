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

impl Default for Configuration {
    fn default() -> Self {
        Self {
            pre: Vec::new(),
            post: Vec::new(),
        }
    }
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
    ContextGetWorkingDirectory = 0x00,
    ContextInvalidate = 0x01,
    ContextAppendArgument = 0x02,

    ParameterGetInteger = 0x10,
    ParameterGetSwitch = 0x11,
    ParameterGetKeyword = 0x12,

    WorkspaceGetPtr = 0x20,
}

impl TryFrom<ffi::c_int> for Interface {
    type Error = ();

    fn try_from(value: ffi::c_int) -> Result<Self, Self::Error> {
        match value {
            x if x == Interface::ContextGetWorkingDirectory as ffi::c_int => {
                Ok(Interface::ContextGetWorkingDirectory)
            }
            x if x == Interface::ContextInvalidate as ffi::c_int => {
                Ok(Interface::ContextInvalidate)
            }
            x if x == Interface::ContextAppendArgument as ffi::c_int => {
                Ok(Interface::ContextAppendArgument)
            }
            x if x == Interface::ParameterGetInteger as ffi::c_int => {
                Ok(Interface::ParameterGetInteger)
            }
            x if x == Interface::ParameterGetSwitch as ffi::c_int => {
                Ok(Interface::ParameterGetSwitch)
            }
            x if x == Interface::ParameterGetKeyword as ffi::c_int => {
                Ok(Interface::ParameterGetKeyword)
            }
            x if x == Interface::WorkspaceGetPtr as ffi::c_int => Ok(Interface::WorkspaceGetPtr),
            _ => Err(()),
        }
    }
}

extern "C" fn get(id: ffi::c_int) -> *const ffi::c_void {
    match Interface::try_from(id) {
        Ok(Interface::ContextGetWorkingDirectory) => {
            context_get_working_directory as *const ffi::c_void
        }
        Ok(Interface::ContextInvalidate) => context_invalidate as *const ffi::c_void,
        Ok(Interface::ContextAppendArgument) => context_append_argument as *const ffi::c_void,
        Ok(Interface::ParameterGetInteger) => parameter_get_integer as *const ffi::c_void,
        Ok(Interface::ParameterGetSwitch) => parameter_get_switch as *const ffi::c_void,
        Ok(Interface::ParameterGetKeyword) => parameter_get_keyword as *const ffi::c_void,
        Ok(Interface::WorkspaceGetPtr) => workspace_get_ptr as *const ffi::c_void,
        _ => ptr::null(),
    }
}

extern "C" fn context_get_working_directory(ctx: *mut Context, ptr: *mut ffi::c_char, size: usize) {
    let ctx = if let Some(ctx) = unsafe { ctx.as_ref() } {
        ctx
    } else {
        return;
    };
    let len = ctx.working_directory.len().min(size - 1);
    unsafe {
        ptr.copy_from_nonoverlapping(ctx.working_directory.as_ptr() as _, len);
        *ptr.add(len) = 0;
    }
}

extern "C" fn context_invalidate(ctx: *mut Context) {
    let ctx = if let Some(ctx) = unsafe { ctx.as_mut() } {
        ctx
    } else {
        return;
    };
    ctx.result = context::Result::Invalid;
}

extern "C" fn context_append_argument(ctx: *mut Context, argument: *const ffi::c_char) {
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

fn get_parameter<'a>(
    ctx: &'a Context,
    name: *const ffi::c_char,
) -> Option<(&'a Specification, &'a Value)> {
    let name = unsafe { ffi::CStr::from_ptr(name) }
        .to_string_lossy()
        .intern();
    let specification = ctx.profile.0.get(&name)?;
    let value = ctx.individual.parameters.get(&name)?;
    Some((specification, value))
}

extern "C" fn parameter_get_integer(
    ctx: *mut Context,
    name: *const ffi::c_char,
) -> *const ffi::c_int {
    let ctx = if let Some(ctx) = unsafe { ctx.as_ref() } {
        ctx
    } else {
        return ptr::null();
    };
    let parameter = if let Some(parameter) = get_parameter(ctx, name) {
        parameter
    } else {
        return ptr::null();
    };
    match parameter {
        (
            Specification::Integer {
                transformer: _,
                space: IntegerSpace::Sequence(_, _),
            },
            Value::Integer(v),
        ) => v as *const ffi::c_int,
        (
            Specification::Integer {
                transformer: _,
                space: IntegerSpace::Candidates(candidates),
            },
            Value::Index(i),
        ) => &candidates[*i] as *const ffi::c_int,
        _ => ptr::null(),
    }
}

static SWITCH_TRUE: ffi::c_int = 1;
static SWITCH_FALSE: ffi::c_int = 0;

extern "C" fn parameter_get_switch(
    ctx: *mut Context,
    name: *const ffi::c_char,
) -> *const ffi::c_int {
    let ctx = if let Some(ctx) = unsafe { ctx.as_ref() } {
        ctx
    } else {
        return ptr::null();
    };
    let parameter = if let Some(parameter) = get_parameter(ctx, name) {
        parameter
    } else {
        return ptr::null();
    };
    match parameter {
        (Specification::Switch, Value::Switch(v)) => {
            if *v {
                &raw const SWITCH_TRUE
            } else {
                &raw const SWITCH_FALSE
            }
        }
        _ => ptr::null(),
    }
}

extern "C" fn parameter_get_keyword(
    ctx: *mut Context,
    name: *const ffi::c_char,
) -> *const ffi::c_char {
    let ctx = if let Some(ctx) = unsafe { ctx.as_ref() } {
        ctx
    } else {
        return ptr::null();
    };
    let parameter = if let Some(parameter) = get_parameter(ctx, name) {
        parameter
    } else {
        return ptr::null();
    };
    match parameter {
        (Specification::Keyword(options), Value::Index(i)) => {
            options.0[*i].as_ptr() as *const ffi::c_char
        }
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
