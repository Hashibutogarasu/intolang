use errors::{Error, Errors, Result};
use std::fmt;
use std::str::FromStr;
use types::{Span, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentLexeme {
    text: String,
    span: Span,
}

impl IdentLexeme {
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

impl FromStr for IdentLexeme {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        let mut chars = text.chars();
        match chars.next() {
            Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
            _ => return Err(Errors::not_lexeme(text, "an identifier").into()),
        }
        if !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
            return Err(Errors::not_lexeme(text, "an identifier").into());
        }
        Ok(Self {
            text: text.to_owned(),
            span: Span::on_line(1),
        })
    }
}

impl TryFrom<&str> for IdentLexeme {
    type Error = Error;

    fn try_from(text: &str) -> Result<Self> {
        text.parse()
    }
}

impl fmt::Display for IdentLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl From<IdentLexeme> for Token {
    fn from(lexeme: IdentLexeme) -> Self {
        Token {
            kind: TokenKind::Identifier(lexeme.text),
            span: lexeme.span,
        }
    }
}
