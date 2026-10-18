use anyhow::{Error, Result, bail};
use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;
use types::Literal;

/// The decoded body of a quoted string literal (`\r` is also recognised).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringBody(String);

impl StringBody {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for StringBody {
    type Err = Infallible;

    fn from_str(raw: &str) -> Result<Self, Infallible> {
        let mut output = String::new();
        let mut chars = raw.chars();
        while let Some(ch) = chars.next() {
            if ch != '\\' {
                output.push(ch);
                continue;
            }
            match chars.next() {
                Some('n') => output.push('\n'),
                Some('r') => output.push('\r'),
                Some('t') => output.push('\t'),
                Some('"') => output.push('"'),
                Some('\\') => output.push('\\'),
                Some(other) => {
                    output.push('\\');
                    output.push(other);
                }
                None => output.push('\\'),
            }
        }
        Ok(Self(output))
    }
}

impl From<&str> for StringBody {
    fn from(raw: &str) -> Self {
        match raw.parse() {
            Ok(body) => body,
            Err(never) => match never {},
        }
    }
}

impl From<StringBody> for String {
    fn from(body: StringBody) -> Self {
        body.0
    }
}

impl fmt::Display for StringBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A literal value written in source: a quoted string, `true`/`false`, a
/// float (contains `.`) or an integer.
#[derive(Debug, Clone, PartialEq)]
pub struct LiteralLexeme(Literal);

impl FromStr for LiteralLexeme {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        let text = text.trim();
        if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
            let body = StringBody::from(&text[1..text.len() - 1]);
            return Ok(Self(Literal::String(body.into())));
        }
        if text == "true" {
            return Ok(Self(Literal::Bool(true)));
        }
        if text == "false" {
            return Ok(Self(Literal::Bool(false)));
        }
        if text.contains('.') {
            if let Ok(value) = text.parse::<f64>() {
                return Ok(Self(Literal::Float(value)));
            }
        }
        if let Ok(value) = text.parse::<i64>() {
            return Ok(Self(Literal::Int(value)));
        }
        bail!("`{text}` is not a literal")
    }
}

impl TryFrom<&str> for LiteralLexeme {
    type Error = Error;

    fn try_from(text: &str) -> Result<Self> {
        text.parse()
    }
}

impl From<LiteralLexeme> for Literal {
    fn from(lexeme: LiteralLexeme) -> Self {
        lexeme.0
    }
}
