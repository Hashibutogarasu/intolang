use anyhow::{Error, Result, anyhow};
use native::FunctionCall;
use parser::Record;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_CALL_FN`: call a function defined by `NATIVE_FUNCTION_BEGIN`.
pub struct NativeCallFn {
    call: FunctionCall,
}

impl TryFrom<&Record> for NativeCallFn {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            call: FunctionCall::try_from(record)?,
        })
    }
}

impl Codegen for NativeCallFn {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let name = CString::new(self.call.name.as_str())?;
            let function = LLVMGetNamedFunction(ctx.module, name.as_ptr());
            if function.is_null() {
                return Err(anyhow!(
                    "Call of the undefined function `{}`",
                    self.call.name
                ));
            }
            let function_type = LLVMGlobalGetValueType(function);
            let (_, mut values) = ctx.native_call_arguments(&self.call.args)?;
            let result = LLVMBuildCall2(
                ctx.builder,
                function_type,
                function,
                values.as_mut_ptr(),
                values.len() as u32,
                ctx.empty.as_ptr(),
            );
            if !self.call.target.is_empty() {
                let ty = ctx.into_type(&self.call.return_type);
                ctx.store_slot(&self.call.target, ty, result);
            }
            Ok(CodegenResult::unit())
        }
    }
}
