use errors::Error;
use std::fmt;
use std::str::FromStr;
use types::{Span, Token, TokenKind};

use crate::StringBody;

/// A string token built from the raw text between its quotes, decoded with
/// the same escape rules as [`StringBody`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLexeme {
    value: String,
    span: Span,
}

impl StringLexeme {
    pub fn at_line(mut self, line: usize) -> Self {
        self.span = Span::on_line(line);
        self
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl FromStr for StringLexeme {
    type Err = Error;

    fn from_str(raw: &str) -> Result<Self, Error> {
        Ok(Self {
            value: StringBody::from(raw).into(),
            span: Span::on_line(1),
        })
    }
}

impl TryFrom<&str> for StringLexeme {
    type Error = Error;

    fn try_from(raw: &str) -> Result<Self, Error> {
        raw.parse()
    }
}

impl fmt::Display for StringLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

impl From<StringLexeme> for Token {
    fn from(lexeme: StringLexeme) -> Self {
        Token {
            kind: TokenKind::String(lexeme.value),
            span: lexeme.span,
        }
    }
}
