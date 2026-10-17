use anyhow::Result;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// A call produced by `#[link]`: `NATIVE_LINK_ASSIGN`.
pub struct NativeLinkAssign {
    target: String,
    return_type: String,
    assigned_to: String,
    args: Vec<String>,
}

impl NativeLinkAssign {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE_LINK_ASSIGN\t";

    pub(crate) fn parse(line: &str) -> Option<Self> {
        let payload = line.strip_prefix(Self::TAG)?.trim_end_matches('\n');
        let mut parts = payload.split('\t');
        Some(Self {
            target: parts.nth(3).unwrap_or_default().to_owned(),
            return_type: parts.next().unwrap_or_default().to_owned(),
            assigned_to: parts.next().unwrap_or_default().to_owned(),
            args: parts.map(str::to_owned).collect(),
        })
    }
}

impl Codegen for NativeLinkAssign {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let (mut params, mut args) =
                ctx.native_call_arguments(self.args.iter().map(String::as_str))?;
            let result_type = if self.return_type == "Void" || self.return_type.is_empty() {
                LLVMVoidTypeInContext(ctx.context)
            } else {
                ctx.into_type(&self.return_type)
            };
            let function_type =
                LLVMFunctionType(result_type, params.as_mut_ptr(), args.len() as u32, 0);
            let target = CString::new(self.target.as_str())?;
            let result = ctx.call(&target, function_type, args.as_mut_ptr(), args.len() as u32);
            if result_type != LLVMVoidTypeInContext(ctx.context) && !self.assigned_to.is_empty() {
                ctx.store_slot(&self.assigned_to, result_type, result);
            }
            Ok(CodegenResult::unit())
        }
    }
}
