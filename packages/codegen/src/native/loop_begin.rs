use anyhow::{Error, Result};
use native::LinkCall;
use parser::Record;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_LOOP_BEGIN`: open a loop whose condition is a native call.
pub struct NativeLoopBegin {
    call: LinkCall,
}

impl TryFrom<&Record> for NativeLoopBegin {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            call: LinkCall::try_from(record)?,
        })
    }
}

impl Codegen for NativeLoopBegin {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let call = &self.call;
            let condition_block = ctx.append_block("while.condition")?;
            let body_block = ctx.append_block("while.body")?;
            let exit_block = ctx.append_block("while.exit")?;
            LLVMBuildBr(ctx.builder, condition_block);
            LLVMPositionBuilderAtEnd(ctx.builder, condition_block);
            let (mut params, mut function_args) = ctx.native_call_arguments(&call.args)?;
            let expected = ctx.into_type(&call.return_type);
            if expected != LLVMInt1TypeInContext(ctx.context) {
                anyhow::bail!("Native loop condition must return Boolean");
            }
            let function_type =
                LLVMFunctionType(expected, params.as_mut_ptr(), function_args.len() as u32, 0);
            let target = CString::new(call.target.as_str())?;
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
