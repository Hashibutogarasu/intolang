use anyhow::{Result, anyhow};
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_CLASS`: a struct type with named, typed fields.
pub struct Class {
    name: String,
    fields: Vec<(String, String)>,
}

impl Class {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE_CLASS\t";

    pub(crate) fn parse(line: &str) -> Option<Self> {
        let spec = line.strip_prefix(Self::TAG)?.trim_end_matches('\n');
        let mut parts = spec.split('\t');
        Some(Self {
            name: parts.next().unwrap_or_default().to_owned(),
            fields: parts
                .map(|part| {
                    let (field, ty) = part.split_once(':').unwrap_or((part, "String"));
                    (field.to_owned(), ty.to_owned())
                })
                .collect(),
        })
    }

    /// Create the opaque named struct before any field types are resolved, so
    /// that classes may reference one another.
    pub(crate) unsafe fn declare(&self, ctx: &mut CodegenContext) -> Result<()> {
        unsafe {
            let name = CString::new(self.name.as_str())?;
            let ty = LLVMStructCreateNamed(ctx.context, name.as_ptr());
            ctx.named_types.insert(self.name.clone(), ty);
            Ok(())
        }
    }
}

impl Codegen for Class {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            let mut fields = Vec::new();
            let mut field_types = Vec::new();
            for (field, type_name) in &self.fields {
                let llvm_type = ctx.into_type(type_name);
                fields.push((field.clone(), llvm_type));
                field_types.push(llvm_type);
            }
            let struct_type = *ctx
                .named_types
                .get(&self.name)
                .ok_or_else(|| anyhow!("Unknown native class `{}`", self.name))?;
            LLVMStructSetBody(
                struct_type,
                field_types.as_mut_ptr(),
                field_types.len() as u32,
                0,
            );
            ctx.class_fields.insert(self.name.clone(), fields);
            Ok(CodegenResult::unit())
        }
    }
}
