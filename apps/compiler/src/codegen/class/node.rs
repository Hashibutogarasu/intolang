use anyhow::{Result, anyhow};
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// A single class field: `name:Type[:<kind>:<value>]`.
struct FieldSpec {
    name: String,
    ty: String,
    default: Option<(char, String)>,
}

/// `NATIVE_CLASS`: a struct type with named, typed fields.
pub struct Class {
    name: String,
    fields: Vec<FieldSpec>,
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
                    let mut segments = part.splitn(4, ':');
                    let name = segments.next().unwrap_or_default().to_owned();
                    let ty = segments.next().unwrap_or("String").to_owned();
                    let kind = segments.next().unwrap_or("-").to_owned();
                    let value = segments.next().unwrap_or_default().to_owned();
                    let default = match kind.as_str() {
                        "-" | "" => None,
                        _ => Some((kind.chars().next().unwrap_or('-'), value)),
                    };
                    FieldSpec { name, ty, default }
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
            let mut defaults = Vec::new();
            for field in &self.fields {
                let llvm_type = ctx.into_type(&field.ty);
                fields.push((field.name.clone(), llvm_type));
                field_types.push(llvm_type);
                defaults.push(field.default.clone());
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
            ctx.class_defaults.insert(self.name.clone(), defaults);
            Ok(CodegenResult::unit())
        }
    }
}
