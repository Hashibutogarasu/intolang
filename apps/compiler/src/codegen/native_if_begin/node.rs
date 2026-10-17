use anyhow::Result;

use crate::llvm;
use llvm::core::*;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_IF_BEGIN`: an `if` on a native comparison.
pub struct NativeIfBegin {
    comparison: String,
}

impl NativeIfBegin {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE_IF_BEGIN\t";

    pub(crate) fn parse(line: &str) -> Option<Self> {
        let payload = line.strip_prefix(Self::TAG)?.trim_end_matches('\n');
        Some(Self {
            comparison: payload.to_owned(),
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
