use errors::{Error, Result};
use native::CompareAssign;
use parser::Record;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_COMPARE`: store the result of a comparison in a `Boolean` slot.
pub struct NativeCompare {
    compare: CompareAssign,
}

impl TryFrom<&Record> for NativeCompare {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            compare: CompareAssign::try_from(record)?,
        })
    }
}

impl Codegen for NativeCompare {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let value = ctx.build_comparison(&self.compare.comparison)?;
            let boolean = ctx.into_type("Boolean");
            ctx.store_slot(&self.compare.target, boolean, value);
            Ok(CodegenResult::unit())
        }
    }
}
