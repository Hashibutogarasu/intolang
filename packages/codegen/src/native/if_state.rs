use errors::{Error, Errors, Result};
use native::StateCheck;
use parser::Record;

use crate::{Codegen, CodegenContext, CodegenResult};

/// `NATIVE_IF_STATE`: a branch on the ownership or memory state of a binding,
/// which only the VM knows while it runs.
pub struct NativeIfState {
    check: StateCheck,
}

impl TryFrom<&Record> for NativeIfState {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            check: StateCheck::try_from(record)?,
        })
    }
}

impl Codegen for NativeIfState {
    fn generate(&self, _ctx: &mut CodegenContext) -> Result<CodegenResult> {
        Err(errors::Error::from(Errors::IfStateNeedsVm {
            slot: self.check.slot.clone(),
            property: self.check.property.to_string(),
        }))
    }
}
