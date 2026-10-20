use errors::Result;
use types::{Literal, Punct};

use crate::{Cursor, Expr, Located, TypeRef};

/// A function parameter: `name`, `name: Type`, `name: Type?`,
/// `name: Type = default` or `name: Type...`.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub name_span: Located,
    pub ty: Option<TypeRef>,
    pub optional: bool,
    pub variadic: bool,
    pub default: Option<Expr>,
}

/// The name a function gets when several functions share `base` and differ
/// in the types of their parameters: `base$Int32$String`. A parameter without
/// a written type is `_`.
pub fn overload_key(base: &str, params: &[Param]) -> String {
    let mut key = format!("{base}$");
    for (index, param) in params.iter().enumerate() {
        if index > 0 {
            key.push('$');
        }
        key.push_str(param.ty.as_ref().map_or("_", TypeRef::as_str));
    }
    key
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
    type Error = errors::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        let (name, _) = cursor.expect_ident("a parameter name")?;
        let name_span = cursor.previous_span();
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
            name_span,
            ty,
            optional,
            variadic,
            default,
        })
    }
}
