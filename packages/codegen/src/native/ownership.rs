use anyhow::{Error, Result};
use native::OwnershipChange;
use parser::Record;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_OWNERSHIP`: what the VM tracks about a binding. The backend has
/// already checked it statically, so compiled code has nothing to do.
pub struct NativeOwnership;

impl TryFrom<&Record> for NativeOwnership {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        OwnershipChange::try_from(record)?;
        Ok(Self)
    }
}

impl Codegen for NativeOwnership {
    fn generate(&self, _ctx: &mut CodegenContext) -> Result<CodegenResult> {
        Ok(CodegenResult::unit())
    }
}
