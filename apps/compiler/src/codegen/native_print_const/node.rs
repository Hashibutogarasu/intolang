use anyhow::Result;

use crate::codegen::{Codegen, CodegenContext, CodegenResult};

/// Print a constant: `NATIVE_PRINT_CONST`.
pub struct NativePrintConst {
    return_type: String,
    operand: String,
    newline: bool,
}

impl NativePrintConst {
    pub(crate) const TAG: &'static str = "\u{1e}NATIVE_PRINT_CONST\t";

    pub(crate) fn parse(line: &str) -> Option<Self> {
        let payload = line.strip_prefix(Self::TAG)?.trim_end_matches('\n');
        let mut parts = payload.split('\t');
        Some(Self {
            return_type: parts.next().unwrap_or_default().to_owned(),
            operand: parts.next().unwrap_or_default().to_owned(),
            newline: parts.next().unwrap_or_default() == "1",
        })
    }
}

impl Codegen for NativePrintConst {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.flush_text()?;
            let ty = ctx.into_type(&self.return_type);
            let (_, value) = self
                .operand
                .split_once(':')
                .unwrap_or(("", self.operand.as_str()));
            let constant = ctx.const_for_type(ty, value)?;
            ctx.print_scalar(ty, constant, self.newline)?;
            Ok(CodegenResult::unit())
        }
    }
}
