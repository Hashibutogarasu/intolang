use anyhow::Result;
use types::{Literal, Punct};

use crate::{Cursor, Expr, TypeRef};

/// A function parameter: `name`, `name: Type`, `name: Type?`,
/// `name: Type = default` or `name: Type...`.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Option<TypeRef>,
    pub optional: bool,
    pub variadic: bool,
    pub default: Option<Expr>,
}

impl Param {
    /// The argument used when a call omits this parameter: its written
    /// default (`true`/`false` become `1`/`0`), or the empty string or `0` for
    /// an optional (`Type?`) parameter.
    pub fn default_argument(&self) -> Option<Expr> {
        match &self.default {
            Some(Expr::Literal(Literal::Bool(flag))) => Some(Expr::int(i64::from(*flag))),
            Some(expression) => Some(expression.clone()),
            None if self.optional => Some(match self.ty.as_ref().map(TypeRef::as_str) {
                Some("String" | "string" | "str") => Expr::Literal(Literal::String(String::new())),
                _ => Expr::int(0),
            }),
            None => None,
        }
    }
}

impl TryFrom<&mut Cursor<'_>> for Param {
    type Error = anyhow::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        let (name, _) = cursor.expect_ident("a parameter name")?;
        let mut ty = None;
        let mut optional = false;
        let mut variadic = false;
        if cursor.eat_punct(Punct::Colon) {
            ty = Some(TypeRef::try_from(&mut *cursor)?);
            if cursor.peek_symbol(0) == Some('?') {
                cursor.bump();
                optional = true;
            }
            while cursor.is_punct(Punct::Dot) && cursor.adjacent(1) {
                cursor.bump();
                variadic = true;
            }
        }
        let default = if cursor.eat_punct(Punct::Assign) {
            Some(Expr::try_from(&mut *cursor)?)
        } else {
            None
        };
        Ok(Self {
            name,
            ty,
            optional,
            variadic,
            default,
        })
    }
}
