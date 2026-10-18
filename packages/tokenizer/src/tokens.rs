use anyhow::{Error, Result};
use std::ops::Deref;
use std::str::FromStr;
use types::Token;

use crate::scan::scan;

/// The token stream of one Into source text.
#[derive(Debug, Clone, PartialEq)]
pub struct Tokens(Vec<Token>);

impl Tokens {
    /// Tokenize `source` keeping `//` comments as [`types::TokenKind::Comment`]
    /// tokens, for tools such as the formatter that must preserve them.
    pub fn with_comments(source: &str) -> Result<Self> {
        scan(source, true).map(Self)
    }
}

impl FromStr for Tokens {
    type Err = Error;

    fn from_str(source: &str) -> Result<Self> {
        scan(source, false).map(Self)
    }
}

impl TryFrom<&str> for Tokens {
    type Error = Error;

    fn try_from(source: &str) -> Result<Self> {
        source.parse()
    }
}

impl Deref for Tokens {
    type Target = [Token];

    fn deref(&self) -> &[Token] {
        &self.0
    }
}

impl From<Tokens> for Vec<Token> {
    fn from(tokens: Tokens) -> Self {
        tokens.0
    }
}

impl IntoIterator for Tokens {
    type Item = Token;
    type IntoIter = std::vec::IntoIter<Token>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Tokens {
    type Item = &'a Token;
    type IntoIter = std::slice::Iter<'a, Token>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
