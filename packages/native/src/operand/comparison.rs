use anyhow::{Error, Result, anyhow};
use parser::{ComparisonLexeme, Record};
use types::{ComparisonOperator, MessageTag};

use super::operand::Operand;

/// The condition of `NATIVE_IF_BEGIN` and `NATIVE_WHILE_BEGIN`: a native slot
/// compared with an operand, at the type the slot holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    pub left: String,
    pub operator: ComparisonOperator,
    pub operand: Operand,
    pub type_name: String,
}

impl Comparison {
    /// The record that opens a branch or loop with this condition.
    pub fn to_record(&self, tag: MessageTag) -> Record {
        Record::new(tag)
            .field(&self.left)
            .field(ComparisonLexeme::from(self.operator))
            .field(&self.operand)
            .field(&self.type_name)
    }
}

impl TryFrom<&Record> for Comparison {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let [left, operator, operand, type_name, ..] = record.fields() else {
            return Err(anyhow!("Malformed comparison record"));
        };
        Ok(Self {
            left: left.clone(),
            operator: operator.parse::<ComparisonLexeme>()?.into(),
            operand: operand.parse()?,
            type_name: type_name.clone(),
        })
    }
}
