use anyhow::Result;

use crate::{Codegen, CodegenContext, CodegenResult};

/// Literal text carried through the message and printed verbatim.
pub struct Text {
    line: String,
}

impl Text {
    pub(crate) fn new(line: &str) -> Self {
        Self {
            line: line.to_owned(),
        }
    }
}

impl Codegen for Text {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        ctx.text.push_str(&self.line);
        Ok(CodegenResult::unit())
    }
}
