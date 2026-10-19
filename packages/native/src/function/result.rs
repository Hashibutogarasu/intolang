use anyhow::{Error, Result, anyhow};
use parser::Record;
use types::MessageTag;

use crate::operand::Operand;

/// `NATIVE_RETURN`: leave the running function. The value, when there is
/// one, has type `ty`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionReturn {
    pub ty: String,
    pub operand: Option<Operand>,
}

impl FunctionReturn {
    pub fn to_record(&self) -> Record {
        let record = Record::new(MessageTag::Return).field(&self.ty);
        match &self.operand {
            Some(operand) => record.field(operand),
            None => record.field(""),
        }
    }
}

impl TryFrom<&Record> for FunctionReturn {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let [ty, operand, ..] = record.fields() else {
            return Err(anyhow!("Malformed NATIVE_RETURN record"));
        };
        Ok(Self {
            ty: ty.clone(),
            operand: if operand.is_empty() {
                None
            } else {
                Some(operand.parse()?)
            },
        })
    }
}
