use anyhow::Result;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::codegen::support::decode_hex;
use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// A plain call to an imported native function: `NATIVE`.
pub struct NativeCall {
    symbol: String,
    kind: String,
    bytes: String,
}

impl NativeCall {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE\t";

    pub(crate) fn parse(line: &str) -> Option<Self> {
        let payload = line.strip_prefix(Self::TAG)?.trim_end_matches('\n');
        let mut parts = payload.split('\t');
        Some(Self {
            symbol: parts.next().unwrap_or_default().to_owned(),
            kind: parts.next().unwrap_or_default().to_owned(),
            bytes: parts.next().unwrap_or_default().to_owned(),
        })
    }
}

impl Codegen for NativeCall {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let symbol = CString::new(self.symbol.as_str())?;
            let argument = if self.kind == "s" {
                let decoded = decode_hex(&self.bytes)?;
                let value = CString::new(decoded)?;
                LLVMBuildGlobalString(ctx.builder, value.as_ptr(), ctx.empty.as_ptr())
            } else {
                let value = self.bytes.parse::<i32>()?;
                LLVMConstInt(LLVMInt32TypeInContext(ctx.context), value as u64, 1)
            };
            let argument_type = if self.kind == "s" {
                LLVMPointerType(LLVMInt8TypeInContext(ctx.context), 0)
            } else {
                LLVMInt32TypeInContext(ctx.context)
            };
            let mut params = vec![argument_type];
            let function_type = LLVMFunctionType(
                LLVMVoidTypeInContext(ctx.context),
                params.as_mut_ptr(),
                1,
                0,
            );
            let mut args = vec![argument];
            ctx.call(&symbol, function_type, args.as_mut_ptr(), 1);
            Ok(CodegenResult::unit())
        }
    }
}
