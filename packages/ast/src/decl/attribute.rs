use errors::Result;
use types::Punct;

use crate::{Cursor, Expr};

/// An attribute such as `#[test]` or `#[link("c", "src/a.c", "symbol")]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub line: usize,
    pub name: String,
    pub args: Vec<Expr>,
}

impl Attribute {
    /// Parse every attribute at the cursor.
    pub fn parse_all(cursor: &mut Cursor<'_>) -> Result<Vec<Attribute>> {
        let mut attributes = Vec::new();
        while cursor.is_punct(Punct::Hash) {
            attributes.push(Attribute::try_from(&mut *cursor)?);
        }
        Ok(attributes)
    }

    /// The string literal arguments, in order, when all arguments are strings.
    pub fn string_args(&self) -> Option<Vec<&str>> {
        self.args
            .iter()
            .map(|arg| match arg {
                Expr::Literal(types::Literal::String(value)) => Some(value.as_str()),
                _ => None,
            })
            .collect()
    }
}

impl TryFrom<&mut Cursor<'_>> for Attribute {
    type Error = errors::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        let line = cursor.line();
        cursor.expect_punct(Punct::Hash, "`#`")?;
        cursor.expect_punct(Punct::LeftBracket, "`[` after `#`")?;
        let (name, _) = cursor.expect_ident("an attribute name")?;
        let mut args = Vec::new();
        if cursor.eat_punct(Punct::LeftParen) && !cursor.eat_punct(Punct::RightParen) {
            loop {
                args.push(Expr::try_from(&mut *cursor)?);
                if cursor.eat_punct(Punct::Comma) {
                    continue;
                }
                cursor.expect_punct(Punct::RightParen, "`)` to close the attribute")?;
                break;
            }
        }
        cursor.expect_punct(Punct::RightBracket, "`]` to close the attribute")?;
        Ok(Self { line, name, args })
    }
}
