use anyhow::{Error, Result, anyhow};
use parser::Record;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::codegen::support::decode_hex;
use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// Implicit construction of a field-only class: `NATIVE_CONSTRUCT`.
pub struct NativeConstruct {
    assigned_to: String,
    class: String,
}

impl TryFrom<&Record> for NativeConstruct {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            assigned_to: record.get(0).to_owned(),
            class: record.get(1).to_owned(),
        })
    }
}

impl Codegen for NativeConstruct {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let struct_type = *ctx
                .named_types
                .get(&self.class)
                .ok_or_else(|| anyhow!("Unknown native class `{}`", self.class))?;
            let fields = ctx
                .class_fields
                .get(&self.class)
                .cloned()
                .unwrap_or_default();
            let mut aggregate = LLVMConstNull(struct_type);
            if let Some(defaults) = ctx.class_defaults.get(&self.class).cloned() {
                for (index, default) in defaults.iter().enumerate() {
                    let Some((kind, value)) = default else {
                        continue;
                    };
                    let Some((_, field_type)) = fields.get(index) else {
                        continue;
                    };
                    let constant = if *kind == 's' {
                        let decoded = decode_hex(value)?;
                        let text = CString::new(decoded)?;
                        LLVMBuildGlobalString(ctx.builder, text.as_ptr(), ctx.empty.as_ptr())
                    } else {
                        ctx.const_for_type(*field_type, value)?
                    };
                    aggregate = LLVMBuildInsertValue(
                        ctx.builder,
                        aggregate,
                        constant,
                        index as u32,
                        ctx.empty.as_ptr(),
                    );
                }
            }
            let storage_name = format!("__value_{}", self.assigned_to);
            let storage = ctx.slot_ptr(&storage_name, struct_type);
            LLVMBuildStore(ctx.builder, aggregate, storage);
            ctx.store_slot(&self.assigned_to, LLVMPointerType(struct_type, 0), storage);
            Ok(CodegenResult::unit())
        }
    }
}
