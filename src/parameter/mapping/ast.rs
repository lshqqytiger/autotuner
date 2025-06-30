use crate::llvm::{
    core::{LLVMBuildAdd, LLVMBuildMul, LLVMConstInt, LLVMInt32TypeInContext},
    prelude::{LLVMBuilderRef, LLVMContextRef, LLVMValueRef},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Expression {
    Integer(i32),
    X,
    Add(Arc<Expression>, Arc<Expression>),
    Mul(Arc<Expression>, Arc<Expression>),
}

impl Expression {
    pub fn into_ir(
        &self,
        context: LLVMContextRef,
        builder: LLVMBuilderRef,
        x: LLVMValueRef,
    ) -> LLVMValueRef {
        unsafe {
            match self {
                Expression::Integer(x) => {
                    let type_i32 = LLVMInt32TypeInContext(context);
                    LLVMConstInt(type_i32, *x as _, 0)
                }
                Expression::X => x,
                Expression::Add(a, b) => {
                    let a = a.into_ir(context, builder, x);
                    let b = b.into_ir(context, builder, x);
                    LLVMBuildAdd(builder, a, b, c"temp_add".as_ptr())
                }
                Expression::Mul(a, b) => {
                    let a = a.into_ir(context, builder, x);
                    let b = b.into_ir(context, builder, x);
                    LLVMBuildMul(builder, a, b, c"temp_mul".as_ptr())
                }
            }
        }
    }
}
