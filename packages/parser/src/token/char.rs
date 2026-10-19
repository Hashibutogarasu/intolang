use anyhow::{Error, bail};
use std::fmt;
use std::str::FromStr;
use types::{Span, Token, TokenKind};

use crate::StringBody;

/// A character token built from the raw text between its single quotes,
/// decoded with the escape rules of [`StringBody`] plus `\'`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharLexeme {
    value: char,
    span: Span,
}

impl CharLexeme {
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    pub fn value(&self) -> char {
        self.value
    }
}

impl FromStr for CharLexeme {
    type Err = Error;

    fn from_str(raw: &str) -> Result<Self, Error> {
        let decoded = StringBody::from(raw.replace("\\'", "'").as_str());
        let mut chars = decoded.as_str().chars();
        match (chars.next(), chars.next()) {
            (Some(value), None) => Ok(Self {
                value,
                span: Span::on_line(1),
            }),
            _ => bail!("A character literal holds exactly one character: '{raw}'"),
        }
    }
}

impl TryFrom<&str> for CharLexeme {
    type Error = Error;

    fn try_from(raw: &str) -> Result<Self, Error> {
        raw.parse()
    }
}

impl fmt::Display for CharLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl From<CharLexeme> for Token {
    fn from(lexeme: CharLexeme) -> Self {
        Token {
            kind: TokenKind::Char(lexeme.value),
            span: lexeme.span,
        }
    }
}
