use errors::{Error, Errors, Result};
use parser::Record;
use types::MessageTag;

use super::property::Property;

/// The condition of `NATIVE_IF_STATE`: a property of a binding compared with
/// a Boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateCheck {
    pub slot: String,
    pub property: Property,
    pub expected: bool,
}

impl StateCheck {
    pub fn to_record(&self) -> Record {
        Record::new(MessageTag::IfState)
            .field(&self.slot)
            .field(self.property)
            .field(if self.expected { "1" } else { "0" })
    }
}

impl TryFrom<&Record> for StateCheck {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let [slot, property, expected, ..] = record.fields() else {
            return Err(Errors::malformed_record("NATIVE_IF_STATE").into());
        };
        Ok(Self {
            slot: slot.clone(),
            property: property.parse()?,
            expected: expected == "1",
        })
    }
}
