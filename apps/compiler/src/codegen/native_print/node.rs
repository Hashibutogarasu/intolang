use anyhow::Result;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// Print a stored native value: `NATIVE_PRINT`.
pub struct NativePrint {
    variable: String,
    return_type: String,
    newline: bool,
}

impl NativePrint {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE_PRINT\t";

    pub(crate) fn parse(line: &str) -> Option<Self> {
        let payload = line.strip_prefix(Self::TAG)?.trim_end_matches('\n');
        let mut parts = payload.split('\t');
        Some(Self {
            variable: parts.next().unwrap_or_default().to_owned(),
            return_type: parts.next().unwrap_or_default().to_owned(),
            newline: parts.next().unwrap_or_default() == "1",
        })
    }
}

impl Codegen for NativePrint {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let ty = ctx.into_type(&self.return_type);
            let value = ctx.load_slot(&self.variable)?;
            ctx.print_scalar(ty, value, self.newline)?;
            Ok(CodegenResult::unit())
        }
    }
}
