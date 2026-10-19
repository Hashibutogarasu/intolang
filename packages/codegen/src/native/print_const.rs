use anyhow::{Error, Result, bail};
use native::Operand;
use parser::Record;

use crate::{Codegen, CodegenContext, CodegenResult};

/// Print a constant: `NATIVE_PRINT_CONST`.
pub struct NativePrintConst {
    return_type: String,
    operand: Operand,
    newline: bool,
}

impl TryFrom<&Record> for NativePrintConst {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            return_type: record.get(0).to_owned(),
            operand: record.get(1).parse()?,
            newline: record.get(2) == "1",
        })
    }
}

impl Codegen for NativePrintConst {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let ty = ctx.into_type(&self.return_type);
            let Operand::Number(value) = &self.operand else {
                bail!("A printed constant must be a number");
            };
            let constant = ctx.const_for_type(ty, value)?;
            ctx.print_scalar(ty, constant, self.newline, self.return_type == "Char")?;
            Ok(CodegenResult::unit())
        }
    }
}
