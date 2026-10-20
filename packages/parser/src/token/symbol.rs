use errors::{Error, Errors, Result};
use std::fmt;
use std::str::FromStr;
use types::{Span, Token, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolLexeme {
    symbol: char,
    span: Span,
}

impl SymbolLexeme {
    pub fn at_line(mut self, line: usize) -> Self {
        self.span = Span::on_line(line);
        self
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }
}

impl From<char> for SymbolLexeme {
    fn from(symbol: char) -> Self {
        Self {
            symbol,
            span: Span::on_line(1),
        }
    }
}

impl FromStr for SymbolLexeme {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(symbol), None) => Ok(symbol.into()),
            _ => Err(Errors::not_lexeme(text, "a single symbol").into()),
        }
    }
}

impl TryFrom<&str> for SymbolLexeme {
    type Error = Error;

    fn try_from(text: &str) -> Result<Self> {
        text.parse()
    }
}

impl fmt::Display for SymbolLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol)
    }
}

impl From<SymbolLexeme> for Token {
    fn from(lexeme: SymbolLexeme) -> Self {
        Token {
            kind: TokenKind::Symbol(lexeme.symbol),
            span: lexeme.span,
        }
    }
}
