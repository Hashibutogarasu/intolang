use std::ffi::CString;

use crate::llvm;
use llvm::core::*;
use llvm::*;

use crate::CodegenContext;

impl CodegenContext {
    /// Allocate storage for one value of the struct `ty` on the heap, so that
    /// it outlives the function and each construction gets its own.
    pub(crate) unsafe fn allocate_instance(&self, ty: *mut LLVMType) -> *mut LLVMValue {
        unsafe {
            let name = CString::new("malloc").unwrap_or_default();
            let size_type = LLVMInt64TypeInContext(self.context);
            let pointer = LLVMPointerType(LLVMInt8TypeInContext(self.context), 0);
            let mut params = [size_type];
            let function_type = LLVMFunctionType(pointer, params.as_mut_ptr(), 1, 0);
            let existing = LLVMGetNamedFunction(self.module, name.as_ptr());
            let function = if existing.is_null() {
                LLVMAddFunction(self.module, name.as_ptr(), function_type)
            } else {
                existing
            };
            let mut size = [LLVMSizeOf(ty)];
            LLVMBuildCall2(
                self.builder,
                function_type,
                function,
                size.as_mut_ptr(),
                1,
                self.empty.as_ptr(),
            )
        }
    }
}
