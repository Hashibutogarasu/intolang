use anyhow::Result;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::codegen::support::decode_hex;
use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// A store into a native slot: `NATIVE_ASSIGN`.
pub struct NativeAssign {
    target: String,
    return_type: String,
    operand: String,
}

impl NativeAssign {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE_ASSIGN\t";

    pub(crate) fn parse(line: &str) -> Option<Self> {
        let payload = line.strip_prefix(Self::TAG)?.trim_end_matches('\n');
        let mut parts = payload.split('\t');
        Some(Self {
            target: parts.next().unwrap_or_default().to_owned(),
            return_type: parts.next().unwrap_or_default().to_owned(),
            operand: parts.next().unwrap_or_default().to_owned(),
        })
    }
}

impl Codegen for NativeAssign {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            let ty = ctx.into_type(&self.return_type);
            let (kind, value) = self
                .operand
                .split_once(':')
                .unwrap_or(("", self.operand.as_str()));
            let stored = if kind == "p" {
                ctx.load_slot(value)?
            } else if kind == "s" {
                let decoded = decode_hex(value)?;
                let value = CString::new(decoded)?;
                LLVMBuildGlobalString(ctx.builder, value.as_ptr(), ctx.empty.as_ptr())
            } else {
                ctx.const_for_type(ty, value)?
            };
            ctx.store_slot(&self.target, ty, stored);
            Ok(CodegenResult::unit())
        }
    }
}
