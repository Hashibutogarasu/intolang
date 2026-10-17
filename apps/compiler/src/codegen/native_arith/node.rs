use anyhow::Result;

use crate::llvm;
use llvm::LLVMTypeKind;
use llvm::core::*;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// An arithmetic update of a native slot: `NATIVE_ARITH`.
pub struct NativeArith {
    assigned: String,
    left: String,
    operation: String,
    operand: String,
    return_type: String,
}

impl NativeArith {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE_ARITH\t";

    pub(crate) fn parse(line: &str) -> Option<Self> {
        let payload = line.strip_prefix(Self::TAG)?.trim_end_matches('\n');
        let mut parts = payload.split('\t');
        Some(Self {
            assigned: parts.next().unwrap_or_default().to_owned(),
            left: parts.next().unwrap_or_default().to_owned(),
            operation: parts.next().unwrap_or_default().to_owned(),
            operand: parts.next().unwrap_or_default().to_owned(),
            return_type: parts.next().unwrap_or_default().to_owned(),
        })
    }
}

impl Codegen for NativeArith {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            let ty = ctx.into_type(&self.return_type);
            let left = ctx.load_slot(&self.left)?;
            let (kind, value) = self
                .operand
                .split_once(':')
                .unwrap_or(("", self.operand.as_str()));
            let right = if kind == "p" {
                ctx.load_slot(value)?
            } else {
                ctx.const_for_type(ty, value)?
            };
            let is_float = matches!(
                LLVMGetTypeKind(ty),
                LLVMTypeKind::LLVMFloatTypeKind | LLVMTypeKind::LLVMDoubleTypeKind
            );
            let result = if is_float {
                match self.operation.as_str() {
                    "add" => LLVMBuildFAdd(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "sub" => LLVMBuildFSub(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "mul" => LLVMBuildFMul(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "div" => LLVMBuildFDiv(ctx.builder, left, right, ctx.empty.as_ptr()),
                    other => anyhow::bail!("Unsupported arithmetic operation `{other}`"),
                }
            } else {
                match self.operation.as_str() {
                    "add" => LLVMBuildAdd(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "sub" => LLVMBuildSub(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "mul" => LLVMBuildMul(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "div" => LLVMBuildSDiv(ctx.builder, left, right, ctx.empty.as_ptr()),
                    other => anyhow::bail!("Unsupported arithmetic operation `{other}`"),
                }
            };
            ctx.store_slot(&self.assigned, ty, result);
            Ok(CodegenResult::unit())
        }
    }
}
