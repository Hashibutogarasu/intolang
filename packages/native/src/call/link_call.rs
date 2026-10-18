use anyhow::{Error, Result, anyhow};
use parser::{LanguageLexeme, Record};
use types::{Language, MessageTag};

use crate::arg::NativeArg;

/// A call to a function bound by `#[link]`, as carried by `NATIVE_LINK_ASSIGN`
/// and by the call records that open a branch or loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkCall {
    pub package_alias: String,
    pub language: Language,
    pub source_path: String,
    pub target: String,
    pub return_type: String,
    pub assigned_to: String,
    pub args: Vec<NativeArg>,
}

impl LinkCall {
    /// The record for this call under `tag`.
    pub fn to_record(&self, tag: MessageTag) -> Record {
        let mut record = Record::new(tag)
            .field(&self.package_alias)
            .field(LanguageLexeme::from(self.language))
            .field(&self.source_path)
            .field(&self.target)
            .field(&self.return_type)
            .field(&self.assigned_to);
        for arg in &self.args {
            record = record.field(arg);
        }
        record
    }
}

impl TryFrom<&Record> for LinkCall {
    type Error = Error;

    fn try_from(record: &Record) -> Result<Self> {
        let [
            alias,
            language,
            source,
            target,
            return_type,
            assigned_to,
            args @ ..,
        ] = record.fields()
        else {
            return Err(anyhow!("Malformed native call record"));
        };
        Ok(Self {
            package_alias: alias.clone(),
            language: language.parse::<LanguageLexeme>()?.into(),
            source_path: source.clone(),
            target: target.clone(),
            return_type: return_type.clone(),
            assigned_to: assigned_to.clone(),
            args: args
                .iter()
                .map(|arg| arg.parse())
                .collect::<Result<Vec<_>>>()?,
        })
    }
}
