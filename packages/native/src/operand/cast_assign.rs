use errors::{Error, Errors, Result};
use parser::Record;
use types::MessageTag;

/// `NATIVE_CAST`: the value of the slot `source` (of type `from`) converted to
/// the type `to` and stored in `target`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastAssign {
    pub target: String,
    pub source: String,
    pub from: String,
    pub to: String,
}

impl CastAssign {
    pub fn to_record(&self) -> Record {
        Record::new(MessageTag::Cast)
            .field(&self.target)
            .field(&self.source)
            .field(&self.from)
            .field(&self.to)
    }
}

impl TryFrom<&Record> for CastAssign {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let [target, source, from, to, ..] = record.fields() else {
            return Err(Errors::malformed_record("NATIVE_CAST").into());
        };
        Ok(Self {
            target: target.clone(),
            source: source.clone(),
            from: from.clone(),
            to: to.clone(),
        })
    }
}
