use errors::{Error, Errors, NativeStackTrace};
use std::ffi::{CStr, c_char};

use crate::llvm;
use llvm::core::LLVMDisposeMessage;

/// The text of an LLVM message, which is released once it is read; `unknown` stands for a
/// missing message.
///
/// # Safety
///
/// `message` is null or an LLVM message that nothing else releases.
pub unsafe fn read_message(message: *mut c_char, unknown: &str) -> String {
    unsafe {
        if message.is_null() {
            return unknown.to_string();
        }
        let text = CStr::from_ptr(message).to_string_lossy().into_owned();
        LLVMDisposeMessage(message);
        text
    }
}

/// An error of LLVM: what LLVM reported is the native layer of its stack trace.
pub fn llvm_failure(context: &str, description: String) -> Error {
    Error::compiler(Errors::LlvmFailed {
        context: context.to_string(),
    })
    .with_native(NativeStackTrace::new("LLVM").with_output(description))
}
