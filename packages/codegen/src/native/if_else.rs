use anyhow::{Result, anyhow};
use parser::Record;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_IF_ELSE`: switch emission to the `else` branch of the current `if`.
pub struct NativeIfElse;

impl From<&Record> for NativeIfElse {
    fn from(_: &Record) -> Self {
        Self
    }
}

impl Codegen for NativeIfElse {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
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
