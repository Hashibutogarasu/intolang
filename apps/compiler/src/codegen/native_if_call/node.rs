use anyhow::Result;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_IF_CALL`: a native call whose Boolean result opens an `if` block.
pub struct NativeIfCall {
    target: String,
    return_type: String,
    args: Vec<String>,
}

impl NativeIfCall {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE_IF_CALL\t";

    pub(crate) fn parse(line: &str) -> Option<Self> {
        let payload = line.strip_prefix(Self::TAG)?.trim_end_matches('\n');
        let mut parts = payload.split('\t');
        Some(Self {
            target: parts.nth(3).unwrap_or_default().to_owned(),
            return_type: parts.next().unwrap_or_default().to_owned(),
            args: parts.skip(1).map(str::to_owned).collect(),
        })
    }
}

impl Codegen for NativeIfCall {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let (mut params, mut args) =
                ctx.native_call_arguments(self.args.iter().map(String::as_str))?;
            let result_type = ctx.into_type(&self.return_type);
            if result_type != LLVMInt1TypeInContext(ctx.context) {
                anyhow::bail!("Native if condition must return Boolean");
            }
            let function_type =
                LLVMFunctionType(result_type, params.as_mut_ptr(), args.len() as u32, 0);
            let target = CString::new(self.target.as_str())?;
            let condition = ctx.call(&target, function_type, args.as_mut_ptr(), args.len() as u32);
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
