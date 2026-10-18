use anyhow::{Error, Result, anyhow};
use parser::Record;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_HANDLER_BEGIN`: the statements up to `NATIVE_HANDLER_END` form the
/// body of an event handler function. Fields: the function name, the
/// captured variables separated by commas, then one `name:Type` per
/// parameter.
pub struct NativeHandlerBegin {
    name: String,
    captures: Vec<String>,
    params: Vec<(String, String)>,
}

impl TryFrom<&Record> for NativeHandlerBegin {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let captures = record
            .get(1)
            .split(',')
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect();
        let params = record
            .fields()
            .iter()
            .skip(2)
            .map(|field| {
                field
                    .split_once(':')
                    .map(|(name, ty)| (name.to_owned(), ty.to_owned()))
                    .ok_or_else(|| anyhow!("Malformed handler parameter `{field}`"))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            name: record.get(0).to_owned(),
            captures,
            params,
        })
    }
}

impl Codegen for NativeHandlerBegin {
    fn generate(&self, ctx: &mut CodegenContext) -> Result<CodegenResult> {
        unsafe {
            ctx.begin_function(&self.name, &self.params, &self.captures)?;
        }
        Ok(CodegenResult::unit())
    }
}
