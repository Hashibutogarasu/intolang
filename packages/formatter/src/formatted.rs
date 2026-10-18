use anyhow::{Error, Result};
use std::fmt;
use std::str::FromStr;
use tokenizer::Tokens;

use crate::layout::layout;
use crate::options::FormatOptions;
use crate::piece::pieces;

/// Source text laid out in the canonical Into style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formatted(String);

impl Formatted {
    pub fn with_options(source: &str, options: FormatOptions) -> Result<Self> {
        let tokens = Tokens::with_comments(source)?;
        let chars: Vec<char> = source.chars().collect();
        Ok(Self(layout(&pieces(&tokens, &chars), options)))
    }
}

impl FromStr for Formatted {
    type Err = Error;

    fn from_str(source: &str) -> Result<Self> {
        Self::with_options(source, FormatOptions::default())
    }
}

impl TryFrom<&str> for Formatted {
    type Error = Error;

    fn try_from(source: &str) -> Result<Self> {
        source.parse()
    }
}

impl From<Formatted> for String {
    fn from(formatted: Formatted) -> Self {
        formatted.0
    }
}

impl fmt::Display for Formatted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Format Into source with `options`. Layout depends only on the tokens,
/// never on how the input was wrapped. Source that cannot be tokenized is
/// returned unchanged.
pub fn format_source_with(source: &str, options: FormatOptions) -> String {
    match Formatted::with_options(source, options) {
        Ok(formatted) => formatted.into(),
        Err(_) => source.to_owned(),
    }
}

/// Format Into source in the default style.
pub fn format_source(source: &str) -> String {
    format_source_with(source, FormatOptions::default())
}
