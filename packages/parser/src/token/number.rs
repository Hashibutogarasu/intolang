use errors::{Error, Errors, Result};
use std::fmt;
use std::str::FromStr;
use types::{Span, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberLexeme {
    text: String,
    span: Span,
}

impl NumberLexeme {
    pub fn at_line(mut self, line: usize) -> Self {
        self.span = Span::on_line(line);
        self
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl FromStr for NumberLexeme {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        if text.is_empty() || !text.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(Errors::not_lexeme(text, "a number").into());
        }
        Ok(Self {
            text: text.to_owned(),
            span: Span::on_line(1),
        })
    }
}

impl TryFrom<&str> for NumberLexeme {
    type Error = Error;

    fn try_from(text: &str) -> Result<Self> {
        text.parse()
    }
}

impl fmt::Display for NumberLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl From<NumberLexeme> for Token {
    fn from(lexeme: NumberLexeme) -> Self {
        Token {
            kind: TokenKind::Number(lexeme.text),
            span: lexeme.span,
        }
    }
}
