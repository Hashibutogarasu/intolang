use anyhow::{Error, Result, anyhow};
use native::{FunctionRef, IndirectCall};
use parser::Record;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_FUNCTION_REF`: store the address of a function in a slot.
pub struct NativeFunctionRef {
    reference: FunctionRef,
}

impl TryFrom<&Record> for NativeFunctionRef {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            reference: FunctionRef::try_from(record)?,
        })
    }
}

impl Codegen for NativeFunctionRef {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            let name = CString::new(self.reference.name.as_str())?;
            let function = LLVMGetNamedFunction(ctx.module, name.as_ptr());
            if function.is_null() {
                return Err(anyhow!(
                    "Reference to the undefined function `{}`",
                    self.reference.name
                ));
            }
            let pointer = LLVMPointerType(LLVMInt8TypeInContext(ctx.context), 0);
            ctx.store_slot(&self.reference.target, pointer, function);
            Ok(CodegenResult::unit())
        }
    }
}

/// `NATIVE_CALL_INDIRECT`: call the function a slot refers to.
pub struct NativeCallIndirect {
    call: IndirectCall,
}

impl TryFrom<&Record> for NativeCallIndirect {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            call: IndirectCall::try_from(record)?,
        })
    }
}

impl Codegen for NativeCallIndirect {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let function = ctx.load_slot(&self.call.slot)?;
            let (mut types, mut values) = ctx.native_call_arguments(&self.call.args)?;
            let function_type = LLVMFunctionType(
                LLVMVoidTypeInContext(ctx.context),
                types.as_mut_ptr(),
                types.len() as u32,
                0,
            );
            LLVMBuildCall2(
                ctx.builder,
                function_type,
                function,
                values.as_mut_ptr(),
                values.len() as u32,
                ctx.empty.as_ptr(),
            );
            Ok(CodegenResult::unit())
        }
    }
}
