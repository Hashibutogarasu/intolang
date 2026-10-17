use anyhow::Result;

use crate::llvm;
use llvm::core::*;
use llvm::*;

use super::super::CodegenContext;

impl CodegenContext {
    /// Resolve an Into type name to its LLVM representation.
    pub(crate) unsafe fn into_type(&self, name: &str) -> *mut LLVMType {
        unsafe {
            if name.ends_with('*') || name.starts_with("List<") {
                return LLVMPointerType(LLVMInt8TypeInContext(self.context), 0);
            }
            match name {
                "String" => LLVMPointerType(LLVMInt8TypeInContext(self.context), 0),
                "Path" => LLVMPointerType(LLVMInt8TypeInContext(self.context), 0),
                "Pointer" => LLVMPointerType(LLVMInt8TypeInContext(self.context), 0),
                "Boolean" => LLVMInt1TypeInContext(self.context),
                "Int8" => LLVMInt8TypeInContext(self.context),
                "Int16" => LLVMInt16TypeInContext(self.context),
                "Int32" => LLVMInt32TypeInContext(self.context),
                "Int64" => LLVMInt64TypeInContext(self.context),
                "Float32" => LLVMFloatTypeInContext(self.context),
                "Float64" => LLVMDoubleTypeInContext(self.context),
                other => self
                    .named_types
                    .get(other)
                    .map(|ty| LLVMPointerType(*ty, 0))
                    .unwrap_or_else(|| LLVMPointerType(LLVMInt8TypeInContext(self.context), 0)),
            }
        }
    }

    pub(crate) unsafe fn const_for_type(
        &self,
        ty: *mut LLVMType,
        text: &str,
    ) -> Result<*mut LLVMValue> {
        unsafe {
            match LLVMGetTypeKind(ty) {
                LLVMTypeKind::LLVMFloatTypeKind | LLVMTypeKind::LLVMDoubleTypeKind => {
                    let value = text.parse::<f64>()?;
                    Ok(LLVMConstReal(ty, value))
                }
                LLVMTypeKind::LLVMPointerTypeKind => Ok(LLVMConstNull(ty)),
                _ => {
                    let value = text.parse::<i64>()?;
                    Ok(LLVMConstInt(ty, value as u64, 1))
                }
            }
        }
    }
}
