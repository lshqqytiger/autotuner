mod ast;
mod parser;

use crate::llvm::{
    core::{
        LLVMAddFunction, LLVMAppendBasicBlockInContext, LLVMBuildRet, LLVMContextCreate,
        LLVMContextDispose, LLVMCreateBuilderInContext, LLVMDisposeBuilder, LLVMFunctionType,
        LLVMGetParam, LLVMInt32TypeInContext, LLVMModuleCreateWithNameInContext,
        LLVMPositionBuilderAtEnd,
    },
    execution_engine::{
        LLVMCreateExecutionEngineForModule, LLVMDisposeExecutionEngine, LLVMExecutionEngineRef,
        LLVMGetFunctionAddress, LLVMLinkInMCJIT,
    },
    prelude::LLVMContextRef,
    target::{LLVM_InitializeNativeAsmPrinter, LLVM_InitializeNativeTarget},
};
use fxhash::FxHashMap;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    mem,
    sync::{Arc, Mutex},
};

type Function<T> = unsafe extern "C" fn(x: T) -> T;

lazy_static! {
    static ref CACHE: Mutex<FxHashMap<Arc<ast::Expression>, Entry>> =
        Mutex::new(FxHashMap::default());
}

unsafe impl Send for Entry {}

struct Entry(LLVMContextRef, LLVMExecutionEngineRef);

impl Entry {
    fn get<T>(&self) -> Function<T> {
        unsafe {
            let address = LLVMGetFunctionAddress(self.1, c"func".as_ptr());
            mem::transmute(address)
        }
    }
}

impl Drop for Entry {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeExecutionEngine(self.1);
            LLVMContextDispose(self.0);
        }
    }
}

pub struct Mapping(Arc<ast::Expression>);

impl Mapping {
    pub fn new(mapping: String) -> Self {
        let parser = parser::grammar::ExpressionParser::new();
        Mapping(parser.parse(mapping.as_str()).expect("syntax error"))
    }

    pub fn map<T>(&self, value: T) -> T {
        let cache = &mut *CACHE.lock().unwrap();
        if let Some(entry) = cache.get(&self.0) {
            let function = entry.get();
            return unsafe { function(value) };
        }

        let entry = self.compile();
        let function = entry.get();
        cache.insert(self.0.clone(), entry);
        unsafe { function(value) }
    }

    fn compile(&self) -> Entry {
        unsafe {
            let context = LLVMContextCreate();
            let module = LLVMModuleCreateWithNameInContext(c"jit".as_ptr(), context);
            let builder = LLVMCreateBuilderInContext(context);

            let type_i32 = LLVMInt32TypeInContext(context);
            let mut args = [type_i32];
            let type_function = LLVMFunctionType(type_i32, args.as_mut_ptr(), args.len() as _, 0);
            let function = LLVMAddFunction(module, c"func".as_ptr(), type_function);

            let block = LLVMAppendBasicBlockInContext(context, function, c"entry".as_ptr());
            LLVMPositionBuilderAtEnd(builder, block);

            let x = LLVMGetParam(function, 0);
            let result = self.0.into_ir(context, builder, x);
            LLVMBuildRet(builder, result);

            LLVMDisposeBuilder(builder);

            LLVMLinkInMCJIT();
            LLVM_InitializeNativeTarget();
            LLVM_InitializeNativeAsmPrinter();

            let mut engine = mem::zeroed();
            let mut err = mem::zeroed();
            LLVMCreateExecutionEngineForModule(&mut engine, module, &mut err);

            Entry(context, engine)
        }
    }
}

impl Serialize for Mapping {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Mapping {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Mapping(Arc::new(ast::Expression::deserialize(
            deserializer,
        )?)))
    }
}
