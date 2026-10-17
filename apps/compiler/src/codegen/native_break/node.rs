use anyhow::{Result, anyhow};

use crate::llvm;
use llvm::core::*;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_BREAK`: jump out of the innermost loop.
pub struct NativeBreak;

impl NativeBreak {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE_BREAK";

    pub(crate) fn parse(line: &str) -> Option<Self> {
        line.starts_with(Self::TAG).then_some(Self)
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
