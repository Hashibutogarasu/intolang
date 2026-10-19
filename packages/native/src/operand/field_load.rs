use anyhow::{Error, Result, anyhow};
use parser::Record;
use types::MessageTag;

/// `NATIVE_FIELD_LOAD`: read the field `index` of the instance of `class`
/// that the slot `slot` holds into the slot `target`, as a value of type `ty`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldLoad {
    pub target: String,
    pub slot: String,
    pub class: String,
    pub index: usize,
    pub ty: String,
}

impl FieldLoad {
    pub fn to_record(&self) -> Record {
        Record::new(MessageTag::FieldLoad)
            .field(&self.target)
            .field(&self.slot)
            .field(&self.class)
            .field(self.index.to_string())
            .field(&self.ty)
    }
}

impl TryFrom<&Record> for FieldLoad {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let [target, slot, class, index, ty, ..] = record.fields() else {
            return Err(anyhow!("Malformed NATIVE_FIELD_LOAD record"));
        };
        Ok(Self {
            target: target.clone(),
            slot: slot.clone(),
            class: class.clone(),
            index: index.parse()?,
            ty: ty.clone(),
        })
    }
}
