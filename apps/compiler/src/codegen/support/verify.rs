use anyhow::{Result, anyhow};
use std::ffi::CStr;
use std::ptr;

use crate::llvm;
use llvm::analysis::{LLVMVerifierFailureAction, LLVMVerifyModule};
use llvm::core::*;
use llvm::*;

use super::super::CodegenContext;

impl CodegenContext {
    /// Close `main`, flush any trailing text and verify the module.
    pub(crate) unsafe fn finish(&mut self) -> Result<()> {
        unsafe {
            self.flush_text()?;
            LLVMBuildRet(
                self.builder,
                LLVMConstInt(LLVMInt64TypeInContext(self.context), 0, 1),
            );
            verify(self.module, "Failed to verify main module")
        }
    }
}

pub(crate) unsafe fn verify(module: *mut LLVMModule, context: &str) -> Result<()> {
    unsafe {
        let mut error = ptr::null_mut();
        if LLVMVerifyModule(
            module,
            LLVMVerifierFailureAction::LLVMPrintMessageAction,
            &mut error,
        ) != 0
        {
            let description = if error.is_null() {
                "unknown verifier error".into()
            } else {
                CStr::from_ptr(error).to_string_lossy().into_owned()
            };
            if !error.is_null() {
                LLVMDisposeMessage(error);
            }
            return Err(anyhow!("{context}: {description}"));
        }
        Ok(())
    }
}
