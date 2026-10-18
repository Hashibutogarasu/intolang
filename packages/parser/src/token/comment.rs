use anyhow::{Error, Result, bail};
use std::fmt;
use std::str::FromStr;
use types::{Span, Token, TokenKind};

/// A `//` line comment, kept verbatim (without its line break).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentLexeme {
    text: String,
    span: Span,
}

impl CommentLexeme {
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl FromStr for CommentLexeme {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        if !text.starts_with("//") || text.contains('\n') {
            bail!("`{text}` is not a line comment");
        }
        Ok(Self {
            text: text.to_owned(),
            span: Span::on_line(1),
        })
    }
}

impl TryFrom<&str> for CommentLexeme {
    type Error = Error;

    fn try_from(text: &str) -> Result<Self> {
        text.parse()
    }
}

impl fmt::Display for CommentLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl From<CommentLexeme> for Token {
    fn from(lexeme: CommentLexeme) -> Self {
        Token {
            kind: TokenKind::Comment(lexeme.text),
            span: lexeme.span,
        }
    }
}
