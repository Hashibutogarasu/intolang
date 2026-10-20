use errors::{Error, Result};
use parser::Record;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_EVENT_DESTROY`: unregister every handler of an event. Field: the
/// event.
pub struct NativeEventDestroy {
    event: String,
}

impl TryFrom<&Record> for NativeEventDestroy {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            event: record.get(0).to_owned(),
        })
    }
}

impl Codegen for NativeEventDestroy {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let table = ctx.event_table(&self.event)?;
            let integer = LLVMInt32TypeInContext(ctx.context);
            let pointer = LLVMPointerType(LLVMInt8TypeInContext(ctx.context), 0);
            let count = LLVMBuildLoad2(ctx.builder, integer, table.count, ctx.empty.as_ptr());
            ctx.counted_loop(count, |ctx, index| {
                let handler_slot = ctx.array_entry(table.handlers_type, table.handlers, index);
                LLVMBuildStore(ctx.builder, LLVMConstNull(pointer), handler_slot);
                Ok(())
            })?;
            Ok(CodegenResult::unit())
        }
    }
}
