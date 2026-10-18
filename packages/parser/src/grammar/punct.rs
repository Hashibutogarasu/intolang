use anyhow::{Error, Result, bail};
use std::fmt;
use std::str::FromStr;
use types::Punct;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PunctLexeme(Punct);

impl TryFrom<char> for PunctLexeme {
    type Error = Error;

    fn try_from(symbol: char) -> Result<Self> {
        Ok(Self(match symbol {
            '(' => Punct::LeftParen,
            ')' => Punct::RightParen,
            '{' => Punct::LeftBrace,
            '}' => Punct::RightBrace,
            '[' => Punct::LeftBracket,
            ']' => Punct::RightBracket,
            ';' => Punct::Semicolon,
            ',' => Punct::Comma,
            ':' => Punct::Colon,
            '.' => Punct::Dot,
            '#' => Punct::Hash,
            '=' => Punct::Assign,
            other => bail!("`{other}` is not a punctuation symbol"),
        }))
    }
}

impl FromStr for PunctLexeme {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(symbol), None) => symbol.try_into(),
            _ => bail!("`{text}` is not a punctuation symbol"),
        }
    }
}

impl From<Punct> for PunctLexeme {
    fn from(punct: Punct) -> Self {
        Self(punct)
    }
}

impl From<PunctLexeme> for Punct {
    fn from(lexeme: PunctLexeme) -> Self {
        lexeme.0
    }
}

impl From<PunctLexeme> for char {
    fn from(lexeme: PunctLexeme) -> Self {
        match lexeme.0 {
            Punct::LeftParen => '(',
            Punct::RightParen => ')',
            Punct::LeftBrace => '{',
            Punct::RightBrace => '}',
            Punct::LeftBracket => '[',
            Punct::RightBracket => ']',
            Punct::Semicolon => ';',
            Punct::Comma => ',',
            Punct::Colon => ':',
            Punct::Dot => '.',
            Punct::Hash => '#',
            Punct::Assign => '=',
        }
    }
}

impl fmt::Display for PunctLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", char::from(*self))
    }
}
