use anyhow::Result;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_LOOP_BEGIN`: open a loop whose condition is a native call.
pub struct NativeLoopBegin {
    target: String,
    return_type: String,
    args: Vec<String>,
}

impl NativeLoopBegin {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE_LOOP_BEGIN\t";

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

impl Codegen for NativeLoopBegin {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let condition_block = ctx.append_block("while.condition")?;
            let body_block = ctx.append_block("while.body")?;
            let exit_block = ctx.append_block("while.exit")?;
            LLVMBuildBr(ctx.builder, condition_block);
            LLVMPositionBuilderAtEnd(ctx.builder, condition_block);
            let (mut params, mut function_args) =
                ctx.native_call_arguments(self.args.iter().map(String::as_str))?;
            let expected = ctx.into_type(&self.return_type);
            if expected != LLVMInt1TypeInContext(ctx.context) {
                anyhow::bail!("Native loop condition must return Boolean");
            }
            let function_type =
                LLVMFunctionType(expected, params.as_mut_ptr(), function_args.len() as u32, 0);
            let target = CString::new(self.target.as_str())?;
            let condition = ctx.call(
                &target,
                function_type,
                function_args.as_mut_ptr(),
                function_args.len() as u32,
            );
            LLVMBuildCondBr(ctx.builder, condition, body_block, exit_block);
            ctx.loop_blocks.push((condition_block, exit_block));
            LLVMPositionBuilderAtEnd(ctx.builder, body_block);
            Ok(CodegenResult::unit())
        }
    }
}
