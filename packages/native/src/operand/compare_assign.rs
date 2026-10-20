use errors::{Error, Errors, Result};
use parser::{ComparisonLexeme, Record};
use types::MessageTag;

use super::comparison::Comparison;

/// `NATIVE_COMPARE`: the result of a comparison, 1 or 0, stored in the
/// `Boolean` slot `target`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareAssign {
    pub target: String,
    pub comparison: Comparison,
}

impl CompareAssign {
    pub fn to_record(&self) -> Record {
        Record::new(MessageTag::Compare)
            .field(&self.target)
            .field(&self.comparison.left)
            .field(ComparisonLexeme::from(self.comparison.operator))
            .field(&self.comparison.operand)
            .field(&self.comparison.type_name)
    }
}

impl TryFrom<&Record> for CompareAssign {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let [target, left, operator, operand, type_name, ..] = record.fields() else {
            return Err(Errors::malformed_record("NATIVE_COMPARE").into());
        };
        Ok(Self {
            target: target.clone(),
            comparison: Comparison {
                left: left.clone(),
                operator: operator.parse::<ComparisonLexeme>()?.into(),
                operand: operand.parse()?,
                type_name: type_name.clone(),
            },
        })
    }
}
