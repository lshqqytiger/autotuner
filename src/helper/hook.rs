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
    temp_dir: &'a [u8],
    pub(crate) sources: Vec<String>,
    pub(crate) arguments: Vec<String>,
    pub(crate) invalidated: bool,
}

impl<'a> Context<'a> {
    pub(crate) fn new(instance: &'a Instance, temp_dir: &'a [u8]) -> Context<'a> {
        Context {
            instance,
            temp_dir,
            sources: Vec::new(),
            arguments: Vec::new(),
            invalidated: false,
        }
    }
}

#[repr(u32)]
pub(crate) enum Function {
    #[allow(dead_code)]
    TempDir = 0x00,
    #[allow(dead_code)]
    GetInt = 0x01,
    #[allow(dead_code)]
    AppendSource = 0x10,
    #[allow(dead_code)]
    AppendArgument = 0x11,
    #[allow(dead_code)]
    Invalidate = 0x20,
}

impl TryFrom<ffi::c_int> for Function {
    type Error = ();

    fn try_from(value: ffi::c_int) -> Result<Self, Self::Error> {
        match value {
            x if x == Function::TempDir as ffi::c_int => Ok(Function::TempDir),
            x if x == Function::GetInt as ffi::c_int => Ok(Function::GetInt),
            x if x == Function::AppendSource as ffi::c_int => Ok(Function::AppendSource),
            x if x == Function::AppendArgument as ffi::c_int => Ok(Function::AppendArgument),
            x if x == Function::Invalidate as ffi::c_int => Ok(Function::Invalidate),
            _ => Err(()),
        }
    }
}

extern "C" fn get(id: ffi::c_int) -> *const ffi::c_void {
    match Function::try_from(id) {
        Ok(Function::TempDir) => temp_dir as *const ffi::c_void,
        Ok(Function::GetInt) => get_int as *const ffi::c_void,
        Ok(Function::AppendSource) => append_source as *const ffi::c_void,
        Ok(Function::AppendArgument) => append_argument as *const ffi::c_void,
        Ok(Function::Invalidate) => invalidate as *const ffi::c_void,
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

extern "C" fn get_int(ctx: *mut Context, name: *const ffi::c_char) -> *const ffi::c_int {
    let ctx = if let Some(ctx) = unsafe { ctx.as_ref() } {
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
    ctx.invalidated = true;
}
