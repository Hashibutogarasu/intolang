use anyhow::{Result, anyhow};
use std::ffi::CString;
use std::ptr;

use crate::llvm;
use llvm::core::*;
use llvm::*;

use crate::CodegenContext;

impl CodegenContext {
    /// The function `pthread_create` runs for the thread function `function`:
    /// it takes and returns an opaque pointer and calls `function`.
    pub(crate) unsafe fn thread_entry(&mut self, function: &str) -> Result<*mut LLVMValue> {
        unsafe {
            let wrapper_name = CString::new(format!("{function}.thread"))?;
            let existing = LLVMGetNamedFunction(self.module, wrapper_name.as_ptr());
            if !existing.is_null() {
                return Ok(existing);
            }
            let target_name = CString::new(function)?;
            let target = LLVMGetNamedFunction(self.module, target_name.as_ptr());
            if target.is_null() {
                return Err(anyhow!("Unknown thread function `{function}`"));
            }
            let pointer = LLVMPointerType(LLVMInt8TypeInContext(self.context), 0);
            let mut params = [pointer];
            let wrapper_type = LLVMFunctionType(pointer, params.as_mut_ptr(), 1, 0);
            let wrapper = LLVMAddFunction(self.module, wrapper_name.as_ptr(), wrapper_type);
            let entry_name = CString::new("entry")?;
            let entry = LLVMAppendBasicBlockInContext(self.context, wrapper, entry_name.as_ptr());
            let saved = LLVMGetInsertBlock(self.builder);
            LLVMPositionBuilderAtEnd(self.builder, entry);
            let target_type =
                LLVMFunctionType(LLVMVoidTypeInContext(self.context), ptr::null_mut(), 0, 0);
            LLVMBuildCall2(
                self.builder,
                target_type,
                target,
                ptr::null_mut(),
                0,
                self.empty.as_ptr(),
            );
            LLVMBuildRet(self.builder, LLVMConstNull(pointer));
            LLVMPositionBuilderAtEnd(self.builder, saved);
            Ok(wrapper)
        }
    }
}
