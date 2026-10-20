use errors::{Error, Errors, Result};
use parser::Record;
use types::MessageTag;

use crate::operand::Operand;

/// `NATIVE_STORE`: write `value`, of type `ty`, as the item `index` of the
/// memory the slot `pointer` holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryStore {
    pub pointer: String,
    pub index: Operand,
    pub ty: String,
    pub value: Operand,
}

impl MemoryStore {
    pub fn to_record(&self) -> Record {
        Record::new(MessageTag::Store)
            .field(&self.pointer)
            .field(&self.index)
            .field(&self.ty)
            .field(&self.value)
    }
}

impl TryFrom<&Record> for MemoryStore {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let [pointer, index, ty, value, ..] = record.fields() else {
            return Err(Errors::malformed_record("NATIVE_STORE").into());
        };
        Ok(Self {
            pointer: pointer.clone(),
            index: index.parse()?,
            ty: ty.clone(),
            value: value.parse()?,
        })
    }
}
