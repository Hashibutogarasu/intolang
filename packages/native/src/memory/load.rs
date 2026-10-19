use anyhow::{Error, Result, anyhow};
use parser::Record;
use types::MessageTag;

use crate::operand::Operand;

/// `NATIVE_LOAD`: read the item `index` of type `ty` from the memory the
/// slot `pointer` holds, into the slot `target`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryLoad {
    pub target: String,
    pub pointer: String,
    pub index: Operand,
    pub ty: String,
}

impl MemoryLoad {
    pub fn to_record(&self) -> Record {
        Record::new(MessageTag::Load)
            .field(&self.target)
            .field(&self.pointer)
            .field(&self.index)
            .field(&self.ty)
    }
}

impl TryFrom<&Record> for MemoryLoad {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let [target, pointer, index, ty, ..] = record.fields() else {
            return Err(anyhow!("Malformed NATIVE_LOAD record"));
        };
        Ok(Self {
            target: target.clone(),
            pointer: pointer.clone(),
            index: index.parse()?,
            ty: ty.clone(),
        })
    }
}
