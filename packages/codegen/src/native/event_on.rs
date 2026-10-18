use anyhow::{Error, Result, anyhow};
use parser::Record;

use crate::llvm;
use llvm::LLVMIntPredicate;
use llvm::core::*;

use crate::support::EVENT_CAPACITY;
use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_EVENT_ON`: register the handler function with an event and store
/// the registration number in the listener slot. Fields: the event, the
/// listener slot (empty when the registration is discarded) and the handler
/// function name.
pub struct NativeEventOn {
    event: String,
    listener: String,
    handler: String,
}

impl TryFrom<&Record> for NativeEventOn {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            event: record.get(0).to_owned(),
            listener: record.get(1).to_owned(),
            handler: record.get(2).to_owned(),
        })
    }
}

impl Codegen for NativeEventOn {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let table = ctx.event_table(&self.event)?;
            let integer = LLVMInt32TypeInContext(ctx.context);
            let handler_name = std::ffi::CString::new(self.handler.as_str())?;
            let handler = LLVMGetNamedFunction(ctx.module, handler_name.as_ptr());
            if handler.is_null() {
                return Err(anyhow!("Unknown event handler `{}`", self.handler));
            }
            if !self.listener.is_empty() {
                ctx.store_slot(&self.listener, integer, LLVMConstInt(integer, 0, 0));
            }
            let count = LLVMBuildLoad2(ctx.builder, integer, table.count, ctx.empty.as_ptr());
            let room = LLVMBuildICmp(
                ctx.builder,
                LLVMIntPredicate::LLVMIntSLT,
                count,
                LLVMConstInt(integer, EVENT_CAPACITY, 0),
                ctx.empty.as_ptr(),
            );
            let register_label = ctx.label("event.register");
            let done_label = ctx.label("event.registered");
            let register = ctx.append_block(&register_label)?;
            let done = ctx.append_block(&done_label)?;
            LLVMBuildCondBr(ctx.builder, room, register, done);

            LLVMPositionBuilderAtEnd(ctx.builder, register);
            let id = LLVMBuildLoad2(ctx.builder, integer, table.next, ctx.empty.as_ptr());
            let handler_slot = ctx.array_entry(table.handlers_type, table.handlers, count);
            LLVMBuildStore(ctx.builder, handler, handler_slot);
            let id_slot = ctx.array_entry(table.ids_type, table.ids, count);
            LLVMBuildStore(ctx.builder, id, id_slot);
            let one = LLVMConstInt(integer, 1, 0);
            let next_count = LLVMBuildAdd(ctx.builder, count, one, ctx.empty.as_ptr());
            LLVMBuildStore(ctx.builder, next_count, table.count);
            let next_id = LLVMBuildAdd(ctx.builder, id, one, ctx.empty.as_ptr());
            LLVMBuildStore(ctx.builder, next_id, table.next);
            if !self.listener.is_empty() {
                ctx.store_slot(&self.listener, integer, id);
            }
            LLVMBuildBr(ctx.builder, done);

            LLVMPositionBuilderAtEnd(ctx.builder, done);
            Ok(CodegenResult::unit())
        }
    }
}
