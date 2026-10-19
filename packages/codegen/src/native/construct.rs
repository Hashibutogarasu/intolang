use anyhow::{Error, Result, anyhow, bail};
use parser::Record;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;
use llvm::*;

use crate::support::decode_hex;
use crate::{Codegen, CodegenContext, CodegenResult};

/// How deep the defaults of fields may build instances inside instances.
const MAX_NESTED_DEFAULTS: usize = 16;

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

/// Build an instance of `class` with the defaults of its fields in storage of
/// its own. A field that starts as another class gets an instance of it.
unsafe fn build_instance(
    ctx: &mut CodegenContext,
    class: &str,
    depth: usize,
) -> Result<*mut LLVMValue> {
    unsafe {
        if depth > MAX_NESTED_DEFAULTS {
            bail!("The defaults of `{class}` build instances without end");
        }
        let struct_type = *ctx
            .named_types
            .get(class)
            .ok_or_else(|| anyhow!("Unknown native class `{class}`"))?;
        let fields = ctx.class_fields.get(class).cloned().unwrap_or_default();
        let mut aggregate = LLVMConstNull(struct_type);
        if let Some(defaults) = ctx.class_defaults.get(class).cloned() {
            for (index, default) in defaults.iter().enumerate() {
                let Some((kind, value)) = default else {
                    continue;
                };
                let Some((_, field_type)) = fields.get(index) else {
                    continue;
                };
                let constant = match kind {
                    's' => {
                        let decoded = decode_hex(value)?;
                        let text = CString::new(decoded)?;
                        LLVMBuildGlobalString(ctx.builder, text.as_ptr(), ctx.empty.as_ptr())
                    }
                    'c' => build_instance(ctx, value, depth + 1)?,
                    _ => ctx.const_for_type(*field_type, value)?,
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
        let storage = ctx.allocate_instance(struct_type);
        LLVMBuildStore(ctx.builder, aggregate, storage);
        Ok(storage)
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
            let storage = build_instance(ctx, &self.class, 0)?;
            ctx.store_slot(&self.assigned_to, LLVMPointerType(struct_type, 0), storage);
            Ok(CodegenResult::unit())
        }
    }
}
