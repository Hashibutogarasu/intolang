use errors::{Error, Result};
use parser::Record;

use crate::{Codegen, CodegenContext, CodegenResult};

/// Print a stored native value: `NATIVE_PRINT`.
pub struct NativePrint {
    variable: String,
    return_type: String,
    newline: bool,
}

impl TryFrom<&Record> for NativePrint {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            variable: record.get(0).to_owned(),
            return_type: record.get(1).to_owned(),
            newline: record.get(2) == "1",
        })
    }
}

impl Codegen for NativePrint {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let ty = ctx.into_type(&self.return_type);
            let value = ctx.load_slot(&self.variable)?;
            ctx.print_scalar(ty, value, self.newline, self.return_type == "Char")?;
            Ok(CodegenResult::unit())
        }
    }
}
