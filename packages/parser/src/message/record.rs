use errors::{Error, Errors, Result};
use std::fmt;
use std::str::FromStr;
use types::{FIELD_SEPARATOR, MessageTag, RECORD_SEPARATOR};

use super::tag::TagLexeme;

/// One compiler directive line of a lowered message: the record separator,
/// a tag, and tab separated fields, ending in a line break.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    tag: MessageTag,
    fields: Vec<String>,
}

impl Record {
    pub fn new(tag: MessageTag) -> Self {
        Self {
            tag,
            fields: Vec::new(),
        }
    }

    pub fn field(mut self, field: impl ToString) -> Self {
        self.fields.push(field.to_string());
        self
    }

    pub fn tag(&self) -> MessageTag {
        self.tag
    }

    pub fn fields(&self) -> &[String] {
        &self.fields
    }

    /// The field at `index`, or the empty string when the record is shorter.
    pub fn get(&self, index: usize) -> &str {
        self.fields.get(index).map_or("", String::as_str)
    }
}

impl FromStr for Record {
    type Err = Error;

    fn from_str(line: &str) -> Result<Self> {
        let Some(body) = line.strip_prefix(RECORD_SEPARATOR) else {
            return Err(Errors::NotDirectiveLine.into());
        };
        let mut parts = body.trim_end_matches('\n').split(FIELD_SEPARATOR);
        let tag = parts.next().unwrap_or_default().parse::<TagLexeme>()?;
        Ok(Self {
            tag: tag.into(),
            fields: parts.map(str::to_owned).collect(),
        })
    }
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{RECORD_SEPARATOR}{}", TagLexeme::from(self.tag))?;
        for field in &self.fields {
            write!(f, "{FIELD_SEPARATOR}{field}")?;
        }
        f.write_str("\n")
    }
}
