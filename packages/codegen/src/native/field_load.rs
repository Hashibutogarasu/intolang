use errors::{Error, Errors, Result};
use native::FieldLoad;
use parser::Record;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_FIELD_LOAD`: read a field of a stored native class value into a
/// slot.
pub struct NativeFieldLoad {
    load: FieldLoad,
}

impl TryFrom<&Record> for NativeFieldLoad {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            load: FieldLoad::try_from(record)?,
        })
    }
}

impl Codegen for NativeFieldLoad {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let class = &self.load.class;
            let field_type = ctx
                .class_fields
                .get(class)
                .and_then(|fields| fields.get(self.load.index))
                .map(|(_, ty)| *ty)
                .ok_or_else(|| {
                    errors::Error::from(Errors::UnknownFieldIndex {
                        class: class.to_string(),
                    })
                })?;
            let object = ctx.load_slot(&self.load.slot)?;
            let struct_type = *ctx.named_types.get(class).ok_or_else(|| {
                errors::Error::from(Errors::UnknownNativeClass {
                    class: class.to_string(),
                })
            })?;
            let field_pointer = LLVMBuildStructGEP2(
                ctx.builder,
                struct_type,
                object,
                self.load.index as u32,
                ctx.empty.as_ptr(),
            );
            let loaded = LLVMBuildLoad2(ctx.builder, field_type, field_pointer, ctx.empty.as_ptr());
            ctx.store_slot(&self.load.target, field_type, loaded);
            Ok(CodegenResult::unit())
        }
    }
}
