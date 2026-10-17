use anyhow::{Result, anyhow};

use crate::llvm;
use llvm::core::*;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_IF_ELSE`: switch emission to the `else` branch of the current `if`.
pub struct NativeIfElse;

impl NativeIfElse {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE_IF_ELSE";

    pub(crate) fn parse(line: &str) -> Option<Self> {
        line.starts_with(Self::TAG).then_some(Self)
    }
}

impl Codegen for NativeIfElse {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            let (else_block, merge_block, has_else) = {
                let last = ctx
                    .if_blocks
                    .last()
                    .ok_or_else(|| anyhow!("`else` has no matching if"))?;
                (last.0, last.1, last.2)
            };
            if has_else {
                anyhow::bail!("An if block cannot contain multiple else branches");
            }
            LLVMBuildBr(ctx.builder, merge_block);
            if let Some(last) = ctx.if_blocks.last_mut() {
                last.2 = true;
            }
            LLVMPositionBuilderAtEnd(ctx.builder, else_block);
            Ok(CodegenResult::unit())
        }
    }
}
