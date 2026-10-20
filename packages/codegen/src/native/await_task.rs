use errors::{Error, Result};
use parser::Record;
use std::ffi::CString;

use crate::llvm;
use llvm::core::*;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_AWAIT`: wait for the thread whose task the slot holds. Field: the
/// slot.
pub struct NativeAwait {
    task: String,
}

impl TryFrom<&Record> for NativeAwait {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            task: record.get(0).to_owned(),
        })
    }
}

impl Codegen for NativeAwait {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let pointer = LLVMPointerType(LLVMInt8TypeInContext(ctx.context), 0);
            let identifier = LLVMInt64TypeInContext(ctx.context);
            let task = ctx.load_slot(&self.task)?;
            let mut params = [identifier, pointer];
            let join_type = LLVMFunctionType(
                LLVMInt32TypeInContext(ctx.context),
                params.as_mut_ptr(),
                2,
                0,
            );
            let mut args = [task, LLVMConstNull(pointer)];
            let name = CString::new("pthread_join")?;
            ctx.call(&name, join_type, args.as_mut_ptr(), 2);
            Ok(CodegenResult::unit())
        }
    }
}
