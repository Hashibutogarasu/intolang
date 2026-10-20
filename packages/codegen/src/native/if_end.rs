use errors::{Errors, Result};
use parser::Record;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_IF_END`: close the current `if` block.
pub struct NativeIfEnd;

impl From<&Record> for NativeIfEnd {
    fn from(_: &Record) -> Self {
        Self
    }
}

impl Codegen for NativeIfEnd {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let (else_block, merge_block, has_else) = ctx
                .if_blocks
                .pop()
                .ok_or_else(|| errors::Error::from(Errors::IfEndWithoutIf))?;
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
