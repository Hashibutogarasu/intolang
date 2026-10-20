use codegen::{llvm_failure, read_message};
use errors::Result;
use std::{ffi::CString, path::Path, ptr};

use crate::llvm;
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
            let description = read_message(error, "unknown object emission error");
            return Err(llvm_failure("Failed to emit object file", description));
        }
        Ok(())
    }
}
