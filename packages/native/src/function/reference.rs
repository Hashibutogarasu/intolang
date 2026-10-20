use errors::{Error, Errors, Result};
use parser::Record;
use types::MessageTag;

use crate::arg::NativeArg;

/// `NATIVE_FUNCTION_REF`: store a reference to the function `name` in the
/// slot `target`, so that it can be passed around and called later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionRef {
    pub target: String,
    pub name: String,
}

impl FunctionRef {
    pub fn to_record(&self) -> Record {
        Record::new(MessageTag::FunctionRef)
            .field(&self.target)
            .field(&self.name)
    }
}

impl TryFrom<&Record> for FunctionRef {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let [target, name, ..] = record.fields() else {
            return Err(Errors::malformed_record("NATIVE_FUNCTION_REF").into());
        };
        Ok(Self {
            target: target.clone(),
            name: name.clone(),
        })
    }
}

/// `NATIVE_CALL_INDIRECT`: call the function that the slot `slot` refers to
/// with `args`, ignoring what it returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndirectCall {
    pub slot: String,
    pub args: Vec<NativeArg>,
}

impl IndirectCall {
    pub fn to_record(&self) -> Record {
        let mut record = Record::new(MessageTag::CallIndirect).field(&self.slot);
        for arg in &self.args {
            record = record.field(arg);
        }
        record
    }
}

impl TryFrom<&Record> for IndirectCall {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let [slot, args @ ..] = record.fields() else {
            return Err(Errors::malformed_record("NATIVE_CALL_INDIRECT").into());
        };
        Ok(Self {
            slot: slot.clone(),
            args: args
                .iter()
                .map(|field| field.parse())
                .collect::<Result<_>>()?,
        })
    }
}
