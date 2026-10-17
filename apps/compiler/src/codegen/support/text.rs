use anyhow::Result;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;
use llvm::*;

use super::super::CodegenContext;

impl CodegenContext {
    /// Emit any text buffered since the last message boundary.
    pub(crate) unsafe fn flush_text(&mut self) -> Result<()> {
        if self.text.is_empty() {
            return Ok(());
        }
        let message = CString::new(self.text.as_str())?;
        self.text.clear();
        unsafe {
            let mut args = vec![LLVMBuildGlobalString(
                self.builder,
                message.as_ptr(),
                self.empty.as_ptr(),
            )];
            LLVMBuildCall2(
                self.builder,
                self.printf_type,
                self.printf,
                args.as_mut_ptr(),
                1,
                self.empty.as_ptr(),
            );
        }
        Ok(())
    }

    pub(crate) unsafe fn print_scalar(
        &self,
        ty: *mut LLVMType,
        value: *mut LLVMValue,
        newline: bool,
    ) -> Result<()> {
        unsafe {
            let (format, value) = match LLVMGetTypeKind(ty) {
                LLVMTypeKind::LLVMFloatTypeKind => {
                    let promoted = LLVMBuildFPExt(
                        self.builder,
                        value,
                        LLVMDoubleTypeInContext(self.context),
                        self.empty.as_ptr(),
                    );
                    (if newline { "%f\n" } else { "%f" }, promoted)
                }
                LLVMTypeKind::LLVMDoubleTypeKind => (if newline { "%f\n" } else { "%f" }, value),
                LLVMTypeKind::LLVMIntegerTypeKind => {
                    let width = LLVMGetIntTypeWidth(ty);
                    if width > 32 {
                        (if newline { "%ld\n" } else { "%ld" }, value)
                    } else {
                        let promoted = if width < 32 {
                            LLVMBuildSExt(
                                self.builder,
                                value,
                                LLVMInt32TypeInContext(self.context),
                                self.empty.as_ptr(),
                            )
                        } else {
                            value
                        };
                        (if newline { "%d\n" } else { "%d" }, promoted)
                    }
                }
                _ => (if newline { "%s\n" } else { "%s" }, value),
            };
            let format = CString::new(format)?;
            let format_value =
                LLVMBuildGlobalString(self.builder, format.as_ptr(), self.empty.as_ptr());
            let mut args = vec![format_value, value];
            LLVMBuildCall2(
                self.builder,
                self.printf_type,
                self.printf,
                args.as_mut_ptr(),
                2,
                self.empty.as_ptr(),
            );
            Ok(())
        }
    }
}
