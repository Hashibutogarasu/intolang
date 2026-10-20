use errors::Result;
use std::ptr;

use crate::llvm;
use llvm::analysis::{LLVMVerifierFailureAction, LLVMVerifyModule};
use llvm::core::*;
use llvm::*;

use super::message::{llvm_failure, read_message};
use crate::CodegenContext;

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
            let description = read_message(error, "unknown verifier error");
            return Err(llvm_failure(context, description));
        }
        Ok(())
    }
}
