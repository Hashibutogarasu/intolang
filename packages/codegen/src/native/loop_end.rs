use errors::{Errors, Result};
use parser::Record;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_LOOP_END`: close the innermost loop.
pub struct NativeLoopEnd;

impl From<&Record> for NativeLoopEnd {
    fn from(_: &Record) -> Self {
        Self
    }
}

impl Codegen for NativeLoopEnd {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let (condition, exit) = ctx
                .loop_blocks
                .pop()
                .ok_or_else(|| errors::Error::from(Errors::LoopEndWithoutLoop))?;
            LLVMBuildBr(ctx.builder, condition);
            LLVMPositionBuilderAtEnd(ctx.builder, exit);
            Ok(CodegenResult::unit())
        }
    }
}
