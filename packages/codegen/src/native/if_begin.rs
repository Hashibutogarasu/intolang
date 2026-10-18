use anyhow::{Error, Result};
use native::Comparison;
use parser::Record;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_IF_BEGIN`: an `if` on a native comparison.
pub struct NativeIfBegin {
    comparison: Comparison,
}

impl TryFrom<&Record> for NativeIfBegin {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            comparison: Comparison::try_from(record)?,
        })
    }
}

impl Codegen for NativeIfBegin {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let condition = ctx.build_comparison(&self.comparison)?;
            let then_block = ctx.append_block("if.then")?;
            let else_block = ctx.append_block("if.else")?;
            let merge_block = ctx.append_block("if.merge")?;
            LLVMBuildCondBr(ctx.builder, condition, then_block, else_block);
            ctx.if_blocks.push((else_block, merge_block, false));
            LLVMPositionBuilderAtEnd(ctx.builder, then_block);
            Ok(CodegenResult::value(condition))
        }
    }
}
