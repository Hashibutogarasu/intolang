use anyhow::{Result, anyhow};
use parser::Record;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_BREAK`: jump out of the innermost loop.
pub struct NativeBreak;

impl From<&Record> for NativeBreak {
    fn from(_: &Record) -> Self {
        Self
    }
}

impl Codegen for NativeBreak {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            let (_, exit) = *ctx
                .loop_blocks
                .last()
                .ok_or_else(|| anyhow!("Native break has no matching loop"))?;
            LLVMBuildBr(ctx.builder, exit);
            let unreachable = ctx.append_block("after.break")?;
            LLVMPositionBuilderAtEnd(ctx.builder, unreachable);
            Ok(CodegenResult::unit())
        }
    }
}
