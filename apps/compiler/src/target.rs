use codegen::{llvm_failure, read_message};
use errors::Result;
use std::ptr;
use std::sync::Once;

use crate::llvm;
use llvm::core::LLVMDisposeMessage;
use llvm::target::{LLVM_InitializeNativeAsmPrinter, LLVM_InitializeNativeTarget};
use llvm::target_machine::*;

/// LLVM registers targets in global tables, so that must happen once even
/// when several programs are compiled concurrently in one process.
static INITIALIZE: Once = Once::new();

pub(super) unsafe fn host_target_machine() -> Result<*mut LLVMOpaqueTargetMachine> {
    unsafe {
        INITIALIZE.call_once(|| {
            LLVM_InitializeNativeTarget();
            LLVM_InitializeNativeAsmPrinter();
        });
        let cpu = LLVMGetHostCPUName();
        let features = LLVMGetHostCPUFeatures();
        let triple = LLVMGetDefaultTargetTriple();

        let mut target = ptr::null_mut();
        let mut error = ptr::null_mut();
        if LLVMGetTargetFromTriple(triple, &mut target, &mut error) != 0 {
            let description = read_message(error, "unknown target error");
            LLVMDisposeMessage(cpu);
            LLVMDisposeMessage(features);
            LLVMDisposeMessage(triple);
            return Err(llvm_failure(
                "Failed to get target from triple",
                description,
            ));
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
