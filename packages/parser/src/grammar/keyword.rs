use errors::{Error, Errors, Result};
use std::fmt;
use std::str::FromStr;
use types::Keyword;

use crate::registry::Registry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordLexeme(Keyword);

impl KeywordLexeme {
    /// Every reserved word, in the order they were registered.
    pub fn all() -> impl Iterator<Item = Self> {
        Registry::global()
            .keywords()
            .iter()
            .map(|definition| Self(definition.keyword))
    }
}

impl FromStr for KeywordLexeme {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        match Registry::global().keyword_spelled(text) {
            Some(definition) => Ok(Self(definition.keyword)),
            None => Err(Errors::not_lexeme(text, "a keyword").into()),
        }
    }
}

impl TryFrom<&str> for KeywordLexeme {
    type Error = Error;

    fn try_from(text: &str) -> Result<Self> {
        text.parse()
    }
}

impl From<Keyword> for KeywordLexeme {
    fn from(keyword: Keyword) -> Self {
        Self(keyword)
    }
}

impl From<KeywordLexeme> for Keyword {
    fn from(lexeme: KeywordLexeme) -> Self {
        lexeme.0
    }
}

impl fmt::Display for KeywordLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let definition = Registry::global()
            .keyword(self.0)
            .expect("every keyword is registered by the bootstrap");
        f.write_str(definition.spelling)
    }
}
