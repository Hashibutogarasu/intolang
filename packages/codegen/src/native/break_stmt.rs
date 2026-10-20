use errors::{Errors, Result};
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
            ctx.flush_text()?;
            let (_, exit) = *ctx
                .loop_blocks
                .last()
                .ok_or_else(|| errors::Error::from(Errors::BreakWithoutLoop))?;
            LLVMBuildBr(ctx.builder, exit);
            let unreachable = ctx.append_block("after.break")?;
            LLVMPositionBuilderAtEnd(ctx.builder, unreachable);
            Ok(CodegenResult::unit())
        }
    }
}
