use anyhow::{Result, anyhow, bail};
use native::{Comparison, NativeArg, Operand};
use parser::Hex;
use std::ffi::CString;
use types::ComparisonOperator;

use crate::llvm;
use llvm::core::*;
use llvm::*;

use super::super::CodegenContext;

impl CodegenContext {
    /// Append a basic block to `main` with the given label.
    pub(crate) unsafe fn append_block(&self, name: &str) -> Result<*mut LLVMBasicBlock> {
        unsafe {
            let label = CString::new(name)?;
            Ok(LLVMAppendBasicBlockInContext(
                self.context,
                self.main,
                label.as_ptr(),
            ))
        }
    }

    /// Declare (if needed) and emit a call to a native function.
    pub(crate) unsafe fn call(
        &self,
        target: &CString,
        function_type: *mut LLVMType,
        args: *mut *mut LLVMValue,
        count: u32,
    ) -> *mut LLVMValue {
        unsafe {
            let existing = LLVMGetNamedFunction(self.module, target.as_ptr());
            let function = if existing.is_null() {
                LLVMAddFunction(self.module, target.as_ptr(), function_type)
            } else {
                existing
            };
            LLVMBuildCall2(
                self.builder,
                function_type,
                function,
                args,
                count,
                self.empty.as_ptr(),
            )
        }
    }

    /// Build the comparison described by an `if`/`while` condition record.
    pub(crate) unsafe fn build_comparison(
        &self,
        comparison: &Comparison,
    ) -> Result<*mut LLVMValue> {
        unsafe {
            let ty = self.into_type(&comparison.type_name);
            let left = self.load_slot(&comparison.left)?;
            let right = match &comparison.operand {
                Operand::Slot(name) => self.load_slot(name)?,
                Operand::Number(value) => self.const_for_type(ty, value)?,
                Operand::Str(_) => bail!("A comparison cannot use a string operand"),
            };
            let is_float = matches!(
                LLVMGetTypeKind(ty),
                LLVMTypeKind::LLVMFloatTypeKind | LLVMTypeKind::LLVMDoubleTypeKind
            );
            let condition = if is_float {
                LLVMBuildFCmp(
                    self.builder,
                    real_predicate(comparison.operator),
                    left,
                    right,
                    self.empty.as_ptr(),
                )
            } else {
                LLVMBuildICmp(
                    self.builder,
                    int_predicate(comparison.operator),
                    left,
                    right,
                    self.empty.as_ptr(),
                )
            };
            Ok(condition)
        }
    }

    pub(crate) unsafe fn native_call_arguments(
        &self,
        args: &[NativeArg],
    ) -> Result<(Vec<*mut LLVMType>, Vec<*mut LLVMValue>)> {
        unsafe {
            let mut params = Vec::new();
            let mut values = Vec::new();
            for arg in args {
                match arg {
                    NativeArg::Str(bytes) => {
                        let value = CString::new(bytes.clone())?;
                        params.push(LLVMPointerType(LLVMInt8TypeInContext(self.context), 0));
                        values.push(LLVMBuildGlobalString(
                            self.builder,
                            value.as_ptr(),
                            self.empty.as_ptr(),
                        ));
                    }
                    NativeArg::Int(number) => {
                        params.push(LLVMInt32TypeInContext(self.context));
                        values.push(LLVMConstInt(
                            LLVMInt32TypeInContext(self.context),
                            *number as u64,
                            1,
                        ));
                    }
                    NativeArg::Slot(name) => {
                        let slot = self.slots.get(name).ok_or_else(|| {
                            anyhow!("Native pointer argument `{name}` has no value")
                        })?;
                        params.push(slot.ty);
                        values.push(LLVMBuildLoad2(
                            self.builder,
                            slot.ty,
                            slot.ptr,
                            self.empty.as_ptr(),
                        ));
                    }
                }
            }
            Ok((params, values))
        }
    }
}

/// Decode the lowercase hexadecimal text a message uses for strings.
pub(crate) fn decode_hex(value: &str) -> Result<Vec<u8>> {
    value.parse::<Hex>()?.try_into()
}

fn int_predicate(operator: ComparisonOperator) -> llvm::LLVMIntPredicate {
    match operator {
        ComparisonOperator::Equal => llvm::LLVMIntPredicate::LLVMIntEQ,
        ComparisonOperator::NotEqual => llvm::LLVMIntPredicate::LLVMIntNE,
        ComparisonOperator::Greater => llvm::LLVMIntPredicate::LLVMIntSGT,
        ComparisonOperator::Less => llvm::LLVMIntPredicate::LLVMIntSLT,
        ComparisonOperator::GreaterEqual => llvm::LLVMIntPredicate::LLVMIntSGE,
        ComparisonOperator::LessEqual => llvm::LLVMIntPredicate::LLVMIntSLE,
    }
}

fn real_predicate(operator: ComparisonOperator) -> llvm::LLVMRealPredicate {
    match operator {
        ComparisonOperator::Equal => llvm::LLVMRealPredicate::LLVMRealOEQ,
        ComparisonOperator::NotEqual => llvm::LLVMRealPredicate::LLVMRealONE,
        ComparisonOperator::Greater => llvm::LLVMRealPredicate::LLVMRealOGT,
        ComparisonOperator::Less => llvm::LLVMRealPredicate::LLVMRealOLT,
        ComparisonOperator::GreaterEqual => llvm::LLVMRealPredicate::LLVMRealOGE,
        ComparisonOperator::LessEqual => llvm::LLVMRealPredicate::LLVMRealOLE,
    }
}
