use errors::Result;
use types::{Keyword, Punct, TokenKind};

use crate::{Cursor, Located, SyntaxError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub name: String,
    pub name_span: Located,
    pub value: Option<i64>,
}

/// `enum Name { A, B = 2 }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDecl {
    pub line: usize,
    pub name: String,
    pub name_span: Located,
    pub variants: Vec<EnumVariant>,
}

impl EnumDecl {
    pub(crate) fn ahead(cursor: &Cursor<'_>) -> bool {
        cursor.peek_keyword(0) == Some(Keyword::Enum) && cursor.peek_word(1).is_some()
    }
}

impl TryFrom<&mut Cursor<'_>> for EnumDecl {
    type Error = errors::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        let line = cursor.line();
        cursor.eat_keyword(Keyword::Enum);
        let (name, _) = cursor.expect_ident("an enum name")?;
        let name_span = cursor.previous_span();
        if !cursor.is_punct(Punct::LeftBrace) {
            return Err(cursor.error_expecting("`{`", "Expected `{` in enum declaration"));
        }
        cursor.bump();
        let mut variants = Vec::new();
        loop {
            if cursor.eat_punct(Punct::RightBrace) {
                break;
            }
            if cursor.at_end() {
                return Err(cursor.error_expecting("`}`", "Expected `}` in enum declaration"));
            }
            let (variant, variant_line) = cursor.expect_ident("an enum variant name")?;
            let variant_span = cursor.previous_span();
            let value = if cursor.eat_punct(Punct::Assign) {
                Some(discriminant(cursor, variant_line)?)
            } else {
                None
            };
            variants.push(EnumVariant {
                name: variant,
                name_span: variant_span,
                value,
            });
            if !cursor.eat_punct(Punct::Comma) && !cursor.is_punct(Punct::RightBrace) {
                return Err(
                    cursor.error_expecting("`,` or `}`", "Expected `,` or `}` in enum declaration")
                );
            }
        }
        Ok(Self {
            line,
            name,
            name_span,
            variants,
        })
    }
}

fn discriminant(cursor: &mut Cursor<'_>, line: usize) -> Result<i64> {
    let negative = cursor.peek_symbol(0) == Some('-');
    if negative {
        cursor.bump();
    }
    match cursor.peek().map(|token| &token.kind) {
        Some(TokenKind::Number(text)) => {
            let value = text.parse::<i64>().map_err(|_| {
                SyntaxError::new(line, "Enum discriminants must be 32-bit integers")
            })?;
            cursor.bump();
            Ok(if negative { -value } else { value })
        }
        _ => Err(SyntaxError::new(line, "Enum discriminants must be 32-bit integers").into()),
    }
}
