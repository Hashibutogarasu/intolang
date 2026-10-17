use anyhow::{Result, anyhow};

use crate::llvm;
use llvm::core::*;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_LOOP_END`: close the innermost loop.
pub struct NativeLoopEnd;

impl NativeLoopEnd {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE_LOOP_END";

    pub(crate) fn parse(line: &str) -> Option<Self> {
        line.starts_with(Self::TAG).then_some(Self)
    }
}

impl Codegen for NativeLoopEnd {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            let (condition, exit) = ctx
                .loop_blocks
                .pop()
                .ok_or_else(|| anyhow!("Native loop end has no matching loop"))?;
            LLVMBuildBr(ctx.builder, condition);
            LLVMPositionBuilderAtEnd(ctx.builder, exit);
            Ok(CodegenResult::unit())
        }
    }
}
