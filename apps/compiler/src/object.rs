use anyhow::{Result, anyhow};
use std::{
    ffi::{CStr, CString},
    path::Path,
    ptr,
};

use crate::llvm;
use llvm::core::LLVMDisposeMessage;
use llvm::target_machine::{
    LLVMCodeGenFileType, LLVMOpaqueTargetMachine, LLVMTargetMachineEmitToFile,
};
use llvm::*;

pub(super) unsafe fn emit(
    module: *mut LLVMModule,
    machine: *mut LLVMOpaqueTargetMachine,
    path: &Path,
) -> Result<()> {
    unsafe {
        let path = CString::new(path.to_string_lossy().as_bytes())?;
        let mut error = ptr::null_mut();
        if LLVMTargetMachineEmitToFile(
            machine,
            module,
            path.as_ptr() as *mut _,
            LLVMCodeGenFileType::LLVMObjectFile,
            &mut error,
        ) != 0
        {
            let description = if error.is_null() {
                "unknown object emission error".into()
            } else {
                CStr::from_ptr(error).to_string_lossy().into_owned()
            };
            if !error.is_null() {
                LLVMDisposeMessage(error);
            }
            return Err(anyhow!("Failed to emit object file: {description}"));
        }
        Ok(())
    }
}
