use errors::{Error, Errors, Result};
use parser::Record;
use types::MessageTag;

use super::operand::Operand;

/// `NATIVE_FIELD_ASSIGN`: store `operand`, a value of type `ty`, in the field
/// `index` of the instance of `class` that the slot `slot` holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldAssign {
    pub slot: String,
    pub class: String,
    pub index: usize,
    pub ty: String,
    pub operand: Operand,
}

impl FieldAssign {
    pub fn to_record(&self) -> Record {
        Record::new(MessageTag::FieldAssign)
            .field(&self.slot)
            .field(&self.class)
            .field(self.index.to_string())
            .field(&self.ty)
            .field(&self.operand)
    }
}

impl TryFrom<&Record> for FieldAssign {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let [slot, class, index, ty, operand, ..] = record.fields() else {
            return Err(Errors::malformed_record("NATIVE_FIELD_ASSIGN").into());
        };
        Ok(Self {
            slot: slot.clone(),
            class: class.clone(),
            index: index.parse()?,
            ty: ty.clone(),
            operand: operand.parse()?,
        })
    }
}
