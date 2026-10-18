use anyhow::{Error, Result, anyhow};
use parser::Record;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::codegen::support::decode_hex;
use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_MODULE`: a resolved module instance.
///
/// The record carries the module index, the module name followed by the fields
/// of the package it belongs to (`alias`, `name`, `version`, `licence`,
/// `root`) and the module source path, all hex encoded. Code generation builds
/// a `PackageInfo` value and stores a `Module` value that references it.
pub struct NativeModule {
    index: String,
    fields: Vec<String>,
}

impl TryFrom<&Record> for NativeModule {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let Some((index, fields)) = record.fields().split_first() else {
            return Err(anyhow!("Malformed NATIVE_MODULE record"));
        };
        Ok(Self {
            index: index.clone(),
            fields: fields.to_vec(),
        })
    }
}

impl Codegen for NativeModule {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let package_type = *ctx
                .named_types
                .get("PackageInfo")
                .ok_or_else(|| anyhow!("Unknown native class `PackageInfo`"))?;
            let module_type = *ctx
                .named_types
                .get("Module")
                .ok_or_else(|| anyhow!("Unknown native class `Module`"))?;
            if self.fields.len() < 7 {
                return Err(anyhow!("Malformed NATIVE_MODULE record"));
            }

            let package_fields = [
                self.fields[1].as_str(),
                self.fields[2].as_str(),
                self.fields[3].as_str(),
                self.fields[4].as_str(),
                self.fields[5].as_str(),
            ];
            let mut package = LLVMConstNull(package_type);
            for (index, encoded) in package_fields.iter().enumerate() {
                let text = CString::new(decode_hex(encoded)?)?;
                let value = LLVMBuildGlobalString(ctx.builder, text.as_ptr(), ctx.empty.as_ptr());
                package = LLVMBuildInsertValue(
                    ctx.builder,
                    package,
                    value,
                    index as u32,
                    ctx.empty.as_ptr(),
                );
            }
            let package_slot =
                ctx.slot_ptr(&format!("__package{}", self.index), package_type);
            LLVMBuildStore(ctx.builder, package, package_slot);

            let mut module = LLVMConstNull(module_type);
            module = LLVMBuildInsertValue(ctx.builder, module, package_slot, 0, ctx.empty.as_ptr());
            let source = CString::new(decode_hex(&self.fields[6])?)?;
            let source_value =
                LLVMBuildGlobalString(ctx.builder, source.as_ptr(), ctx.empty.as_ptr());
            module =
                LLVMBuildInsertValue(ctx.builder, module, source_value, 1, ctx.empty.as_ptr());
            let module_slot = ctx.slot_ptr(&format!("__module{}", self.index), module_type);
            LLVMBuildStore(ctx.builder, module, module_slot);
            Ok(CodegenResult::unit())
        }
    }
}
