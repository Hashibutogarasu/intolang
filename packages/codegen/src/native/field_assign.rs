use errors::{Error, Errors, Result};
use native::{FieldAssign, Operand};
use parser::Record;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_FIELD_ASSIGN`: store a value in a field of a stored native class
/// value.
pub struct NativeFieldAssign {
    assign: FieldAssign,
}

impl TryFrom<&Record> for NativeFieldAssign {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            assign: FieldAssign::try_from(record)?,
        })
    }
}

impl Codegen for NativeFieldAssign {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let class = &self.assign.class;
            let field_type = ctx
                .class_fields
                .get(class)
                .and_then(|fields| fields.get(self.assign.index))
                .map(|(_, ty)| *ty)
                .ok_or_else(|| {
                    errors::Error::from(Errors::UnknownFieldIndex {
                        class: class.to_string(),
                    })
                })?;
            let object = ctx.load_slot(&self.assign.slot)?;
            let struct_type = *ctx.named_types.get(class).ok_or_else(|| {
                errors::Error::from(Errors::UnknownNativeClass {
                    class: class.to_string(),
                })
            })?;
            let field_pointer = LLVMBuildStructGEP2(
                ctx.builder,
                struct_type,
                object,
                self.assign.index as u32,
                ctx.empty.as_ptr(),
            );
            let stored = match &self.assign.operand {
                Operand::Slot(name) => ctx.load_slot(name)?,
                Operand::Str(bytes) => {
                    let value = CString::new(bytes.clone())?;
                    LLVMBuildGlobalString(ctx.builder, value.as_ptr(), ctx.empty.as_ptr())
                }
                Operand::Number(value) => ctx.const_for_type(field_type, value)?,
            };
            LLVMBuildStore(ctx.builder, stored, field_pointer);
            Ok(CodegenResult::unit())
        }
    }
}
