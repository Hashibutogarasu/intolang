use anyhow::{Error, Result};
use native::{MemoryLoad, MemoryStore, Operand};
use parser::Record;

use crate::llvm;
use llvm::core::*;
use llvm::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// The address of item `index` of type `element` in the memory `pointer`
/// holds.
unsafe fn item_address(
    ctx: &CodegenContext,
    pointer: &str,
    index: &Operand,
    element: *mut LLVMType,
) -> Result<*mut LLVMValue> {
    unsafe {
        let base = ctx.load_slot(pointer)?;
        let wide = LLVMInt64TypeInContext(ctx.context);
        let position = match index {
            Operand::Slot(name) => {
                LLVMBuildSExt(ctx.builder, ctx.load_slot(name)?, wide, ctx.empty.as_ptr())
            }
            Operand::Number(number) => LLVMConstInt(wide, number.parse::<i64>()? as u64, 1),
            Operand::Str(_) => anyhow::bail!("An index cannot be text"),
        };
        let mut indices = [position];
        Ok(LLVMBuildGEP2(
            ctx.builder,
            element,
            base,
            indices.as_mut_ptr(),
            1,
            ctx.empty.as_ptr(),
        ))
    }
}

/// `NATIVE_LOAD`: read an item of memory into a slot.
pub struct NativeLoad {
    load: MemoryLoad,
}

impl TryFrom<&Record> for NativeLoad {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            load: MemoryLoad::try_from(record)?,
        })
    }
}

impl Codegen for NativeLoad {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let element = ctx.into_type(&self.load.ty);
            let address = item_address(ctx, &self.load.pointer, &self.load.index, element)?;
            let loaded = LLVMBuildLoad2(ctx.builder, element, address, ctx.empty.as_ptr());
            ctx.store_slot(&self.load.target, element, loaded);
            Ok(CodegenResult::unit())
        }
    }
}

/// `NATIVE_STORE`: write an item of memory.
pub struct NativeStore {
    store: MemoryStore,
}

impl TryFrom<&Record> for NativeStore {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            store: MemoryStore::try_from(record)?,
        })
    }
}

impl Codegen for NativeStore {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let element = ctx.into_type(&self.store.ty);
            let value = match &self.store.value {
                Operand::Slot(name) => ctx.load_slot(name)?,
                Operand::Number(number) => ctx.const_for_type(element, number)?,
                Operand::Str(_) => anyhow::bail!("Text cannot be stored as an item"),
            };
            let address = item_address(ctx, &self.store.pointer, &self.store.index, element)?;
            LLVMBuildStore(ctx.builder, value, address);
            Ok(CodegenResult::unit())
        }
    }
}
