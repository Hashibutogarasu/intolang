use anyhow::{Result, anyhow};

use crate::llvm;
use llvm::core::*;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_IF_END`: close the current `if` block.
pub struct NativeIfEnd;

impl NativeIfEnd {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE_IF_END";

    pub(crate) fn parse(line: &str) -> Option<Self> {
        line.starts_with(Self::TAG).then_some(Self)
    }
}

impl Codegen for NativeIfEnd {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            let (else_block, merge_block, has_else) = ctx
                .if_blocks
                .pop()
                .ok_or_else(|| anyhow!("if end has no matching if"))?;
            LLVMBuildBr(ctx.builder, merge_block);
            if !has_else {
                LLVMPositionBuilderAtEnd(ctx.builder, else_block);
                LLVMBuildBr(ctx.builder, merge_block);
            }
            LLVMPositionBuilderAtEnd(ctx.builder, merge_block);
            Ok(CodegenResult::unit())
        }
    }
}
