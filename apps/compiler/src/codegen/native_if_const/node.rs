use anyhow::{Error, Result};
use parser::Record;

use crate::llvm;
use llvm::core::*;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_IF_CONST`: an `if` on a compile-time Boolean literal.
pub struct NativeIfConst {
    flag: bool,
}

impl TryFrom<&Record> for NativeIfConst {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            flag: record.get(0).trim() == "1",
        })
    }
}

impl Codegen for NativeIfConst {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let condition = LLVMConstInt(
                LLVMInt1TypeInContext(ctx.context),
                if self.flag { 1 } else { 0 },
                0,
            );
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
