use anyhow::{Error, Result, anyhow};
use parser::Record;
use types::MessageTag;

use crate::arg::NativeArg;

/// `NATIVE_CALL_FN`: call the function `name` with `args`. The result of type
/// `return_type` goes to the slot `target`; both are empty for a function
/// that returns nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCall {
    pub name: String,
    pub target: String,
    pub return_type: String,
    pub args: Vec<NativeArg>,
}

impl FunctionCall {
    pub fn to_record(&self) -> Record {
        let mut record = Record::new(MessageTag::CallFn)
            .field(&self.name)
            .field(&self.target)
            .field(&self.return_type);
        for arg in &self.args {
            record = record.field(arg);
        }
        record
    }
}

impl TryFrom<&Record> for FunctionCall {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let [name, target, return_type, args @ ..] = record.fields() else {
            return Err(anyhow!("Malformed NATIVE_CALL_FN record"));
        };
        Ok(Self {
            name: name.clone(),
            target: target.clone(),
            return_type: return_type.clone(),
            args: args
                .iter()
                .map(|field| field.parse())
                .collect::<Result<_>>()?,
        })
    }
}
