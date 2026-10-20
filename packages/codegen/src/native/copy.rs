use errors::{Error, Errors, Result};
use parser::Record;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `copy`: the whole instance is duplicated into storage the target owns.
pub struct NativeCopy {
    target: String,
    source: String,
    class: String,
}

impl TryFrom<&Record> for NativeCopy {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            target: record.get(0).to_owned(),
            source: record.get(1).to_owned(),
            class: record.get(2).to_owned(),
        })
    }
}

impl Codegen for NativeCopy {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let struct_type = *ctx.named_types.get(&self.class).ok_or_else(|| {
                errors::Error::from(Errors::UnknownNativeClass {
                    class: self.class.to_string(),
                })
            })?;
            let source = ctx.load_slot(&self.source)?;
            let instance = LLVMBuildLoad2(ctx.builder, struct_type, source, ctx.empty.as_ptr());
            let storage = ctx.allocate_instance(struct_type);
            LLVMBuildStore(ctx.builder, instance, storage);
            ctx.store_slot(&self.target, LLVMPointerType(struct_type, 0), storage);
            Ok(CodegenResult::unit())
        }
    }
}
