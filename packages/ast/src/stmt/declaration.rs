use anyhow::Result;
use types::{Keyword, Punct, Storage};

use crate::{Cursor, Expr, TypeRef};

/// `[const] [now|late] into [*]name[: Type] [= value];`.
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub storage: Storage,
    pub constant: bool,
    pub pointer: bool,
    pub name: String,
    pub annotation: Option<TypeRef>,
    pub value: Option<Expr>,
}

impl Declaration {
    pub(crate) fn ahead(cursor: &Cursor<'_>) -> bool {
        matches!(
            (
                cursor.peek_keyword(0),
                cursor.peek_keyword(1),
                cursor.peek_keyword(2)
            ),
            (
                Some(Keyword::Const),
                Some(Keyword::Now),
                Some(Keyword::Into)
            ) | (Some(Keyword::Now | Keyword::Late), Some(Keyword::Into), _)
                | (Some(Keyword::Into), _, _)
        )
    }
}

impl TryFrom<&mut Cursor<'_>> for Declaration {
    type Error = anyhow::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        let constant = cursor.eat_keyword(Keyword::Const);
        let storage = if cursor.eat_keyword(Keyword::Now) {
            Storage::Now
        } else if cursor.eat_keyword(Keyword::Late) {
            Storage::Late
        } else {
            Storage::Local
        };
        if !cursor.eat_keyword(Keyword::Into) {
            return Err(cursor.error("Expected `into`"));
        }
        let pointer = cursor.peek_symbol(0) == Some('*');
        if pointer {
            cursor.bump();
        }
        let (name, _) = cursor.expect_ident("a variable name after `into`")?;
        let annotation = if cursor.eat_punct(Punct::Colon) {
            Some(TypeRef::try_from(&mut *cursor)?)
        } else {
            None
        };
        let value = if cursor.eat_punct(Punct::Assign) {
            Some(Expr::try_from(&mut *cursor)?)
        } else {
            None
        };
        cursor.end_statement()?;
        Ok(Self {
            storage,
            constant,
            pointer,
            name,
            annotation,
            value,
        })
    }
}
