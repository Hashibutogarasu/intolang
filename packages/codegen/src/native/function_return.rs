use anyhow::{Error, Result, anyhow};
use native::{FunctionReturn, Operand};
use parser::Record;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_RETURN`: leave the running function with a value.
pub struct NativeReturn {
    result: FunctionReturn,
}

impl TryFrom<&Record> for NativeReturn {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            result: FunctionReturn::try_from(record)?,
        })
    }
}

impl Codegen for NativeReturn {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            match &self.result.operand {
                None => {
                    LLVMBuildRetVoid(ctx.builder);
                }
                Some(operand) => {
                    let ty = ctx.into_type(&self.result.ty);
                    let value = match operand {
                        Operand::Slot(name) => ctx.load_slot(name)?,
                        Operand::Str(bytes) => {
                            let text = CString::new(bytes.clone())?;
                            LLVMBuildGlobalString(ctx.builder, text.as_ptr(), ctx.empty.as_ptr())
                        }
                        Operand::Number(number) => ctx.const_for_type(ty, number)?,
                    };
                    LLVMBuildRet(ctx.builder, value);
                }
            }
            let label = ctx.label("after.return");
            let after = ctx.append_block(&label).map_err(|e| anyhow!("{e}"))?;
            LLVMPositionBuilderAtEnd(ctx.builder, after);
            Ok(CodegenResult::unit())
        }
    }
}
