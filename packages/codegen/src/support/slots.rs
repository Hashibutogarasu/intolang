use anyhow::{Result, anyhow};

use crate::llvm;
use llvm::core::*;
use llvm::*;

use crate::CodegenContext;
use crate::context::Slot;

impl CodegenContext {
    pub(crate) unsafe fn slot_ptr(&mut self, name: &str, ty: *mut LLVMType) -> *mut LLVMValue {
        unsafe {
            if let Some(slot) = self.slots.get(name) {
                return slot.ptr;
            }
            let current = LLVMGetInsertBlock(self.builder);
            let first = LLVMGetFirstInstruction(self.entry);
            if first.is_null() {
                LLVMPositionBuilderAtEnd(self.builder, self.entry);
            } else {
                LLVMPositionBuilderBefore(self.builder, first);
            }
            let ptr = LLVMBuildAlloca(self.builder, ty, self.empty.as_ptr());
            if !current.is_null() {
                LLVMPositionBuilderAtEnd(self.builder, current);
            }
            self.slots.insert(name.to_owned(), Slot { ptr, ty });
            ptr
        }
    }

    pub(crate) unsafe fn store_slot(
        &mut self,
        name: &str,
        ty: *mut LLVMType,
        value: *mut LLVMValue,
    ) {
        unsafe {
            let ptr = self.slot_ptr(name, ty);
            LLVMBuildStore(self.builder, value, ptr);
        }
    }

    pub(crate) unsafe fn load_slot(&self, name: &str) -> Result<*mut LLVMValue> {
        unsafe {
            let slot = self
                .slots
                .get(name)
                .ok_or_else(|| anyhow!("Native value `{name}` has no storage"))?;
            Ok(LLVMBuildLoad2(
                self.builder,
                slot.ty,
                slot.ptr,
                self.empty.as_ptr(),
            ))
        }
    }
}
