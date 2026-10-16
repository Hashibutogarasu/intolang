use anyhow::{Result, anyhow};
use std::{ffi::CStr, ptr};

use crate::llvm;
use llvm::core::LLVMDisposeMessage;
use llvm::target::{LLVM_InitializeNativeAsmPrinter, LLVM_InitializeNativeTarget};
use llvm::target_machine::*;

pub(super) unsafe fn host_target_machine() -> Result<*mut LLVMOpaqueTargetMachine> {
    unsafe {
        LLVM_InitializeNativeTarget();
        LLVM_InitializeNativeAsmPrinter();
        let cpu = LLVMGetHostCPUName();
        let features = LLVMGetHostCPUFeatures();
        let triple = LLVMGetDefaultTargetTriple();

        let mut target = ptr::null_mut();
        let mut error = ptr::null_mut();
        if LLVMGetTargetFromTriple(triple, &mut target, &mut error) != 0 {
            let description = if error.is_null() {
                "unknown target error".into()
            } else {
                CStr::from_ptr(error).to_string_lossy().into_owned()
            };
            if !error.is_null() {
                LLVMDisposeMessage(error);
            }
            LLVMDisposeMessage(cpu);
            LLVMDisposeMessage(features);
            LLVMDisposeMessage(triple);
            return Err(anyhow!("Failed to get target from triple: {description}"));
        }

        let machine = LLVMCreateTargetMachine(
            target,
            triple,
            cpu,
            features,
            LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
            LLVMRelocMode::LLVMRelocPIC,
            LLVMCodeModel::LLVMCodeModelDefault,
        );
        LLVMDisposeMessage(cpu);
        LLVMDisposeMessage(features);
        LLVMDisposeMessage(triple);
        Ok(machine)
    }
}
