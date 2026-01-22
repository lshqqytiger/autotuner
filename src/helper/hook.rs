use crate::{
    parameter::{Instance, Value},
    utils::interner::Intern,
};
use libloading::Symbol;
use std::{ffi, ptr};

type HookFunction = unsafe extern "C" fn(
    ctx: *mut Context,
    get: extern "C" fn(id: ffi::c_int) -> *const ffi::c_void,
);

pub(crate) struct Hook<'a>(Symbol<'a, HookFunction>);

impl<'a> From<Symbol<'a, HookFunction>> for Hook<'a> {
    fn from(f: Symbol<'a, HookFunction>) -> Self {
        Hook(f)
    }
}

impl<'a> Hook<'a> {
    pub(crate) fn call(&self, context: &mut Context) {
        unsafe {
            self.0(context as _, get as _);
        }
    }
}

pub(crate) struct Context<'a> {
    instance: &'a Instance,
    pub(crate) sources: Vec<String>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(instance: &'a Instance) -> Context<'a> {
        Context {
            instance,
            sources: Vec::new(),
        }
    }
}

#[repr(u32)]
pub(crate) enum Function {
    #[allow(dead_code)]
    GetInt = 0x00,
    #[allow(dead_code)]
    AppendSource = 0x10,
}

impl TryFrom<ffi::c_int> for Function {
    type Error = ();

    fn try_from(value: ffi::c_int) -> Result<Self, Self::Error> {
        match value {
            x if x == Function::GetInt as ffi::c_int => Ok(Function::GetInt),
            x if x == Function::AppendSource as ffi::c_int => Ok(Function::AppendSource),
            _ => Err(()),
        }
    }
}

extern "C" fn get(id: ffi::c_int) -> *const ffi::c_void {
    match Function::try_from(id) {
        Ok(Function::GetInt) => get_int as *const ffi::c_void,
        Ok(Function::AppendSource) => append_source as *const ffi::c_void,
        _ => ptr::null(),
    }
}

extern "C" fn get_int(ctx: *mut Context, name: *const ffi::c_char) -> *const ffi::c_int {
    let ctx = if let Some(ctx) = unsafe { ctx.as_mut() } {
        ctx
    } else {
        return ptr::null();
    };
    if let Some(Value::Integer(x)) = ctx.instance.parameters.get(
        &unsafe { ffi::CStr::from_ptr(name) }
            .to_string_lossy()
            .into_owned()
            .intern(),
    ) {
        x as *const i32
    } else {
        ptr::null()
    }
}

extern "C" fn append_source(ctx: *mut Context, path: *const ffi::c_char) {
    let ctx = if let Some(ctx) = unsafe { ctx.as_mut() } {
        ctx
    } else {
        return;
    };
    ctx.sources.push(
        unsafe { ffi::CStr::from_ptr(path) }
            .to_string_lossy()
            .into_owned(),
    );
}
