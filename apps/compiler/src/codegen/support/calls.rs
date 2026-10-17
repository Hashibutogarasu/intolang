use anyhow::{Result, anyhow};
use std::ffi::CString;

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

    /// Build the comparison described by an `if`/`while` condition line.
    pub(crate) unsafe fn build_comparison(&self, call: &str) -> Result<*mut LLVMValue> {
        unsafe {
            let mut parts = call.trim_end_matches('\n').split('\t');
            let left_name = parts.next().unwrap_or_default();
            let operator = parts.next().unwrap_or_default();
            let operand = parts.next().unwrap_or_default();
            let type_name = parts.next().unwrap_or_default();
            let ty = self.into_type(type_name);
            let left = self.load_slot(left_name)?;
            let (kind, value) = operand.split_once(':').unwrap_or(("", operand));
            let right = if kind == "p" {
                self.load_slot(value)?
            } else {
                self.const_for_type(ty, value)?
            };
            let is_float = matches!(
                LLVMGetTypeKind(ty),
                LLVMTypeKind::LLVMFloatTypeKind | LLVMTypeKind::LLVMDoubleTypeKind
            );
            let condition = if is_float {
                LLVMBuildFCmp(
                    self.builder,
                    real_predicate(operator)?,
                    left,
                    right,
                    self.empty.as_ptr(),
                )
            } else {
                LLVMBuildICmp(
                    self.builder,
                    int_predicate(operator)?,
                    left,
                    right,
                    self.empty.as_ptr(),
                )
            };
            Ok(condition)
        }
    }

    pub(crate) unsafe fn native_call_arguments<'a>(
        &self,
        args: impl Iterator<Item = &'a str>,
    ) -> Result<(Vec<*mut LLVMType>, Vec<*mut LLVMValue>)> {
        unsafe {
            let mut params = Vec::new();
            let mut values = Vec::new();
            for arg in args {
                let (kind, value) = arg.split_once(':').unwrap_or(("", ""));
                if kind == "s" {
                    let decoded = decode_hex(value)?;
                    let value = CString::new(decoded)?;
                    params.push(LLVMPointerType(LLVMInt8TypeInContext(self.context), 0));
                    values.push(LLVMBuildGlobalString(
                        self.builder,
                        value.as_ptr(),
                        self.empty.as_ptr(),
                    ));
                } else if kind == "i" {
                    let number = value.parse::<i32>()?;
                    params.push(LLVMInt32TypeInContext(self.context));
                    values.push(LLVMConstInt(
                        LLVMInt32TypeInContext(self.context),
                        number as u64,
                        1,
                    ));
                } else if kind == "p" {
                    let slot = self
                        .slots
                        .get(value)
                        .ok_or_else(|| anyhow!("Native pointer argument `{value}` has no value"))?;
                    params.push(slot.ty);
                    values.push(LLVMBuildLoad2(
                        self.builder,
                        slot.ty,
                        slot.ptr,
                        self.empty.as_ptr(),
                    ));
                } else {
                    anyhow::bail!("Unsupported native link argument type `{kind}`");
                }
            }
            Ok((params, values))
        }
    }
}

pub(crate) fn decode_hex(value: &str) -> Result<Vec<u8>> {
    if value.len() % 2 != 0 {
        anyhow::bail!("Invalid native string argument encoding");
    }
    (0..value.len())
        .step_by(2)
        .map(|index| Ok(u8::from_str_radix(&value[index..index + 2], 16)?))
        .collect()
}

fn int_predicate(operator: &str) -> Result<llvm::LLVMIntPredicate> {
    Ok(match operator {
        "==" => llvm::LLVMIntPredicate::LLVMIntEQ,
        "!=" => llvm::LLVMIntPredicate::LLVMIntNE,
        ">" => llvm::LLVMIntPredicate::LLVMIntSGT,
        "<" => llvm::LLVMIntPredicate::LLVMIntSLT,
        ">=" => llvm::LLVMIntPredicate::LLVMIntSGE,
        "<=" => llvm::LLVMIntPredicate::LLVMIntSLE,
        _ => anyhow::bail!("Unsupported comparison operator `{operator}`"),
    })
}

fn real_predicate(operator: &str) -> Result<llvm::LLVMRealPredicate> {
    Ok(match operator {
        "==" => llvm::LLVMRealPredicate::LLVMRealOEQ,
        "!=" => llvm::LLVMRealPredicate::LLVMRealONE,
        ">" => llvm::LLVMRealPredicate::LLVMRealOGT,
        "<" => llvm::LLVMRealPredicate::LLVMRealOLT,
        ">=" => llvm::LLVMRealPredicate::LLVMRealOGE,
        "<=" => llvm::LLVMRealPredicate::LLVMRealOLE,
        _ => anyhow::bail!("Unsupported comparison operator `{operator}`"),
    })
}
