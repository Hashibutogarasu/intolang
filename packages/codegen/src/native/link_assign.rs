use anyhow::{Error, Result};
use native::LinkCall;
use parser::Record;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// A call produced by `#[link]`: `NATIVE_LINK_ASSIGN`.
pub struct NativeLinkAssign {
    call: LinkCall,
}

impl TryFrom<&Record> for NativeLinkAssign {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            call: LinkCall::try_from(record)?,
        })
    }
}

impl Codegen for NativeLinkAssign {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let call = &self.call;
            let (mut params, mut args) = ctx.native_call_arguments(&call.args)?;
            let result_type = if call.return_type == "Void" || call.return_type.is_empty() {
                LLVMVoidTypeInContext(ctx.context)
            } else {
                ctx.into_type(&call.return_type)
            };
            let function_type =
                LLVMFunctionType(result_type, params.as_mut_ptr(), args.len() as u32, 0);
            let target = CString::new(call.target.as_str())?;
            let result = ctx.call(&target, function_type, args.as_mut_ptr(), args.len() as u32);
            if result_type != LLVMVoidTypeInContext(ctx.context) && !call.assigned_to.is_empty() {
                ctx.store_slot(&call.assigned_to, result_type, result);
            }
            Ok(CodegenResult::unit())
        }
    }
}
