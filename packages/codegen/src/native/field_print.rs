use anyhow::{Error, Result, anyhow};
use parser::Record;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// Print a field of a stored native class value: `NATIVE_FIELD_PRINT`.
pub struct NativeFieldPrint {
    variable: String,
    class_name: String,
    field_index: u32,
    character: bool,
    newline: bool,
}

impl TryFrom<&Record> for NativeFieldPrint {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            variable: record.get(0).to_owned(),
            class_name: record.get(1).to_owned(),
            field_index: record.get(2).parse()?,
            character: record.get(3) == "Char",
            newline: record.get(4) == "1",
        })
    }
}

impl Codegen for NativeFieldPrint {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let field_type = ctx
                .class_fields
                .get(&self.class_name)
                .and_then(|fields| fields.get(self.field_index as usize))
                .map(|(_, ty)| *ty)
                .ok_or_else(|| anyhow!("Unknown field index for class `{}`", self.class_name))?;
            let object = ctx.load_slot(&self.variable)?;
            let struct_type = *ctx
                .named_types
                .get(&self.class_name)
                .ok_or_else(|| anyhow!("Unknown native class `{}`", self.class_name))?;
            let field_pointer = LLVMBuildStructGEP2(
                ctx.builder,
                struct_type,
                object,
                self.field_index,
                ctx.empty.as_ptr(),
            );
            let loaded = LLVMBuildLoad2(ctx.builder, field_type, field_pointer, ctx.empty.as_ptr());
            ctx.print_scalar(field_type, loaded, self.newline, self.character)?;
            Ok(CodegenResult::unit())
        }
    }
}
