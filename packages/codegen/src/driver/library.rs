use anyhow::Result;
use moduler::NativeFunction;
use std::ffi::CString;
use std::ptr;

use crate::llvm;
use llvm::core::*;

use crate::support::verify;

/// Generate an exported native shared library from lowered functions.
///
/// # Safety
///
/// `context`, `module` and `builder` must be valid LLVM objects that belong
/// together.
pub unsafe fn generate_library(
    context: *mut llvm::LLVMContext,
    module: *mut llvm::LLVMModule,
    builder: *mut llvm::LLVMBuilder,
    functions: &[NativeFunction],
) -> Result<()> {
    unsafe {
        let empty = CString::new("")?;
        let mut printf_params = vec![LLVMPointerType(LLVMInt8TypeInContext(context), 0)];
        let printf_type = LLVMFunctionType(
            LLVMInt32TypeInContext(context),
            printf_params.as_mut_ptr(),
            0,
            1,
        );
        let printf_name = CString::new("printf")?;
        let printf = LLVMAddFunction(module, printf_name.as_ptr(), printf_type);
        for exported in functions {
            let function_name = CString::new(exported.name.as_bytes())?;
            let function_type =
                LLVMFunctionType(LLVMVoidTypeInContext(context), ptr::null_mut(), 0, 0);
            let function = LLVMAddFunction(module, function_name.as_ptr(), function_type);
            let block_name = CString::new("entry")?;
            let block = LLVMAppendBasicBlockInContext(context, function, block_name.as_ptr());
            LLVMPositionBuilderAtEnd(builder, block);
            if !exported.output.is_empty() {
                let format_string = CString::new("%s")?;
                let format_value =
                    LLVMBuildGlobalString(builder, format_string.as_ptr(), empty.as_ptr());
                let output = CString::new(exported.output.as_bytes())?;
                let output_value = LLVMBuildGlobalString(builder, output.as_ptr(), empty.as_ptr());
                let mut args = vec![format_value, output_value];
                LLVMBuildCall2(
                    builder,
                    printf_type,
                    printf,
                    args.as_mut_ptr(),
                    2,
                    empty.as_ptr(),
                );
            }
            LLVMBuildRetVoid(builder);
        }
        verify(module, "Failed to verify native library module")
    }
}
