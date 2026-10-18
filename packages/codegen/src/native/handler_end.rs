use anyhow::Result;
use parser::Record;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_HANDLER_END`: close the handler function opened by
/// `NATIVE_HANDLER_BEGIN`.
pub struct NativeHandlerEnd;

impl From<&Record> for NativeHandlerEnd {
    fn from(_: &Record) -> Self {
        Self
    }
}

impl Codegen for NativeHandlerEnd {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.end_function()?;
        }
        Ok(CodegenResult::unit())
    }
}
