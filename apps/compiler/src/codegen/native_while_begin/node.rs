use anyhow::Result;

use crate::llvm;
use llvm::core::*;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_WHILE_BEGIN`: open a loop whose condition is a native comparison.
pub struct NativeWhileBegin {
    comparison: String,
}

impl NativeWhileBegin {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE_WHILE_BEGIN\t";

    pub(crate) fn parse(line: &str) -> Option<Self> {
        let payload = line.strip_prefix(Self::TAG)?.trim_end_matches('\n');
        Some(Self {
            comparison: payload.to_owned(),
        })
    }
}

impl Codegen for NativeWhileBegin {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let condition_block = ctx.append_block("while.condition")?;
            let body_block = ctx.append_block("while.body")?;
            let exit_block = ctx.append_block("while.exit")?;
            LLVMBuildBr(ctx.builder, condition_block);
            LLVMPositionBuilderAtEnd(ctx.builder, condition_block);
            let condition = ctx.build_comparison(&self.comparison)?;
            LLVMBuildCondBr(ctx.builder, condition, body_block, exit_block);
            ctx.loop_blocks.push((condition_block, exit_block));
            LLVMPositionBuilderAtEnd(ctx.builder, body_block);
            Ok(CodegenResult::value(condition))
        }
    }
}
