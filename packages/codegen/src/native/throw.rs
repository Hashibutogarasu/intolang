use errors::{Error, Errors, Result};
use parser::Record;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_THROW`: fail the run with a message. Only the VM can do that.
pub struct NativeThrow {
    message: String,
}

impl TryFrom<&Record> for NativeThrow {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            message: record.get(0).to_owned(),
        })
    }
}

impl Codegen for NativeThrow {
    fn generate(&self, _ctx: &mut CodegenContext) -> Result<CodegenResult> {
        return Err(errors::Error::from(Errors::ThrowNeedsVm {
            message: self.message.to_string(),
        }));
    }
}
