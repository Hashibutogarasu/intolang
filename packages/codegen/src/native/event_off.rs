use anyhow::{Error, Result};
use parser::Record;

use crate::llvm;
use llvm::LLVMIntPredicate;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_EVENT_OFF`: unregister the handler whose registration number the
/// listener slot holds. Fields: the event and the listener slot.
pub struct NativeEventOff {
    event: String,
    listener: String,
}

impl TryFrom<&Record> for NativeEventOff {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            event: record.get(0).to_owned(),
            listener: record.get(1).to_owned(),
        })
    }
}

impl Codegen for NativeEventOff {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let table = ctx.event_table(&self.event)?;
            let integer = LLVMInt32TypeInContext(ctx.context);
            let pointer = LLVMPointerType(LLVMInt8TypeInContext(ctx.context), 0);
            let id = ctx.load_slot(&self.listener)?;
            let count = LLVMBuildLoad2(ctx.builder, integer, table.count, ctx.empty.as_ptr());
            ctx.counted_loop(count, |ctx, index| {
                let id_slot = ctx.array_entry(table.ids_type, table.ids, index);
                let registered = LLVMBuildLoad2(ctx.builder, integer, id_slot, ctx.empty.as_ptr());
                let same = LLVMBuildICmp(
                    ctx.builder,
                    LLVMIntPredicate::LLVMIntEQ,
                    registered,
                    id,
                    ctx.empty.as_ptr(),
                );
                let clear_label = ctx.label("event.clear");
                let keep_label = ctx.label("event.keep");
                let clear = ctx.append_block(&clear_label)?;
                let keep = ctx.append_block(&keep_label)?;
                LLVMBuildCondBr(ctx.builder, same, clear, keep);
                LLVMPositionBuilderAtEnd(ctx.builder, clear);
                let handler_slot = ctx.array_entry(table.handlers_type, table.handlers, index);
                LLVMBuildStore(ctx.builder, LLVMConstNull(pointer), handler_slot);
                LLVMBuildBr(ctx.builder, keep);
                LLVMPositionBuilderAtEnd(ctx.builder, keep);
                Ok(())
            })?;
            Ok(CodegenResult::unit())
        }
    }
}
