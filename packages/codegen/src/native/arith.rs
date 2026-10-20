use errors::{Error, Errors, Result};
use native::Operand;
use parser::Record;

use crate::llvm;
use llvm::LLVMTypeKind;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// An arithmetic update of a native slot: `NATIVE_ARITH`.
pub struct NativeArith {
    assigned: String,
    left: String,
    operation: String,
    operand: Operand,
    return_type: String,
}

impl TryFrom<&Record> for NativeArith {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            assigned: record.get(0).to_owned(),
            left: record.get(1).to_owned(),
            operation: record.get(2).to_owned(),
            operand: record.get(3).parse()?,
            return_type: record.get(4).to_owned(),
        })
    }
}

impl Codegen for NativeArith {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            let ty = ctx.into_type(&self.return_type);
            let left = ctx.load_slot(&self.left)?;
            let right = match &self.operand {
                Operand::Slot(name) => ctx.load_slot(name)?,
                Operand::Number(value) => ctx.const_for_type(ty, value)?,
                Operand::Str(_) => {
                    return Err(errors::Error::from(Errors::ArithmeticStringOperand));
                }
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
                    "rem" => LLVMBuildFRem(ctx.builder, left, right, ctx.empty.as_ptr()),
                    other => {
                        return Err(errors::Error::from(
                            Errors::UnsupportedArithmeticOperation {
                                operation: other.to_string(),
                            },
                        ));
                    }
                }
            } else {
                match self.operation.as_str() {
                    "add" => LLVMBuildAdd(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "sub" => LLVMBuildSub(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "mul" => LLVMBuildMul(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "div" => LLVMBuildSDiv(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "rem" => LLVMBuildSRem(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "and" => LLVMBuildAnd(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "or" => LLVMBuildOr(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "xor" => LLVMBuildXor(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "shl" => LLVMBuildShl(ctx.builder, left, right, ctx.empty.as_ptr()),
                    "shr" => LLVMBuildAShr(ctx.builder, left, right, ctx.empty.as_ptr()),
                    other => {
                        return Err(errors::Error::from(
                            Errors::UnsupportedArithmeticOperation {
                                operation: other.to_string(),
                            },
                        ));
                    }
                }
            };
            ctx.store_slot(&self.assigned, ty, result);
            Ok(CodegenResult::unit())
        }
    }
}
