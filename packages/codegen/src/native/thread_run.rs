use errors::{Error, Result};
use parser::Record;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_THREAD_RUN`: start a thread that runs a function. Fields: the slot
/// that receives the task (empty when it is discarded) and the function.
pub struct NativeThreadRun {
    task: String,
    function: String,
}

impl TryFrom<&Record> for NativeThreadRun {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            task: record.get(0).to_owned(),
            function: record.get(1).to_owned(),
        })
    }
}

impl Codegen for NativeThreadRun {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let entry = ctx.thread_entry(&self.function)?;
            let pointer = LLVMPointerType(LLVMInt8TypeInContext(ctx.context), 0);
            let identifier = LLVMInt64TypeInContext(ctx.context);
            let slot_name = if self.task.is_empty() {
                ctx.label("__thread_task")
            } else {
                self.task.clone()
            };
            let task = ctx.slot_ptr(&slot_name, identifier);
            let mut params = [pointer, pointer, pointer, pointer];
            let create_type = LLVMFunctionType(
                LLVMInt32TypeInContext(ctx.context),
                params.as_mut_ptr(),
                4,
                0,
            );
            let mut args = [task, LLVMConstNull(pointer), entry, LLVMConstNull(pointer)];
            let name = CString::new("pthread_create")?;
            ctx.call(&name, create_type, args.as_mut_ptr(), 4);
            Ok(CodegenResult::unit())
        }
    }
}
