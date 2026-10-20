use errors::{Error, Result};
use native::Operand;
use parser::Record;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// A store into a native slot: `NATIVE_ASSIGN`.
pub struct NativeAssign {
    target: String,
    return_type: String,
    operand: Operand,
}

impl TryFrom<&Record> for NativeAssign {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            target: record.get(0).to_owned(),
            return_type: record.get(1).to_owned(),
            operand: record.get(2).parse()?,
        })
    }
}

impl Codegen for NativeAssign {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            let ty = ctx.into_type(&self.return_type);
            let stored = match &self.operand {
                Operand::Slot(name) => ctx.load_slot(name)?,
                Operand::Str(bytes) => {
                    let value = CString::new(bytes.clone())?;
                    LLVMBuildGlobalString(ctx.builder, value.as_ptr(), ctx.empty.as_ptr())
                }
                Operand::Number(value) => ctx.const_for_type(ty, value)?,
            };
            ctx.store_slot(&self.target, ty, stored);
            Ok(CodegenResult::unit())
        }
    }
}
