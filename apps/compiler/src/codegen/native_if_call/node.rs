use anyhow::{Error, Result};
use native::LinkCall;
use parser::Record;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_IF_CALL`: a native call whose Boolean result opens an `if` block.
pub struct NativeIfCall {
    call: LinkCall,
}

impl TryFrom<&Record> for NativeIfCall {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            call: LinkCall::try_from(record)?,
        })
    }
}

impl Codegen for NativeIfCall {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let call = &self.call;
            let (mut params, mut args) = ctx.native_call_arguments(&call.args)?;
            let result_type = ctx.into_type(&call.return_type);
            if result_type != LLVMInt1TypeInContext(ctx.context) {
                anyhow::bail!("Native if condition must return Boolean");
            }
            let function_type =
                LLVMFunctionType(result_type, params.as_mut_ptr(), args.len() as u32, 0);
            let target = CString::new(call.target.as_str())?;
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
