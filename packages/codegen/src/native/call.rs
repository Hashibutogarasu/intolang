use errors::{Error, Result};
use native::NativeArg;
use parser::Record;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// A plain call to an imported native function: `NATIVE`.
pub struct NativeCall {
    symbol: String,
    argument: NativeArg,
}

impl TryFrom<&Record> for NativeCall {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            symbol: record.get(0).to_owned(),
            argument: format!("{}:{}", record.get(1), record.get(2)).parse()?,
        })
    }
}

impl Codegen for NativeCall {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let symbol = CString::new(self.symbol.as_str())?;
            let (mut params, mut args) =
                ctx.native_call_arguments(std::slice::from_ref(&self.argument))?;
            let function_type = LLVMFunctionType(
                LLVMVoidTypeInContext(ctx.context),
                params.as_mut_ptr(),
                1,
                0,
            );
            ctx.call(&symbol, function_type, args.as_mut_ptr(), 1);
            Ok(CodegenResult::unit())
        }
    }
}
