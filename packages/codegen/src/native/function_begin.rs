use anyhow::{Error, Result};
use native::FunctionDefinition;
use parser::Record;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_FUNCTION_BEGIN`: the statements up to `NATIVE_HANDLER_END` form the
/// body of a function that other statements call and that may return a value.
pub struct NativeFunctionBegin {
    definition: FunctionDefinition,
}

impl TryFrom<&Record> for NativeFunctionBegin {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            definition: FunctionDefinition::try_from(record)?,
        })
    }
}

impl Codegen for NativeFunctionBegin {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.begin_function(
                &self.definition.name,
                &self.definition.params,
                &[],
                Some(&self.definition.return_type),
            )?;
        }
        Ok(CodegenResult::unit())
    }
}
