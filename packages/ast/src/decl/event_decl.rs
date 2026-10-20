use errors::Result;
use types::{Keyword, Punct};

use super::param::Param;
use super::type_params::parse_type_params;
use crate::stmt::{Scope, parse_block};
use crate::{Cursor, Stmt};

/// `[pub] [static] [default|override] event name<T>(params) { body }`: an
/// event whose body is the handler that runs before the registered ones.
#[derive(Debug, Clone, PartialEq)]
pub struct EventDecl {
    pub line: usize,
    pub name: String,
    pub is_public: bool,
    pub is_static: bool,
    pub is_default: bool,
    pub is_override: bool,
    pub type_params: Vec<String>,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
}

impl EventDecl {
    /// Whether modifiers followed by `event` and a name start at the cursor.
    pub(crate) fn ahead(cursor: &Cursor<'_>) -> bool {
        let mut offset = 0;
        loop {
            match cursor.peek_keyword(offset) {
                Some(Keyword::Pub | Keyword::Static | Keyword::Default | Keyword::Override) => {
                    offset += 1;
                }
                Some(Keyword::Event) => return cursor.peek_word(offset + 1).is_some(),
                _ => return false,
            }
        }
    }
}

impl TryFrom<&mut Cursor<'_>> for EventDecl {
    type Error = errors::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        let line = cursor.line();
        let (mut is_public, mut is_static, mut is_default, mut is_override) =
            (false, false, false, false);
        loop {
            if cursor.eat_keyword(Keyword::Pub) {
                is_public = true;
            } else if cursor.eat_keyword(Keyword::Static) {
                is_static = true;
            } else if cursor.eat_keyword(Keyword::Default) {
                is_default = true;
            } else if cursor.eat_keyword(Keyword::Override) {
                is_override = true;
            } else {
                break;
            }
        }
        if !cursor.eat_keyword(Keyword::Event) {
            return Err(cursor.error("Expected `event`"));
        }
        let (name, _) = cursor.expect_ident("an event name")?;
        let type_params = parse_type_params(cursor)?;
        cursor.expect_punct(Punct::LeftParen, "`(` after the event name")?;
        let mut params = Vec::new();
        if !cursor.eat_punct(Punct::RightParen) {
            loop {
                params.push(Param::try_from(&mut *cursor)?);
                if cursor.eat_punct(Punct::Comma) {
                    continue;
                }
                cursor.expect_punct(Punct::RightParen, "`)` to close the parameters")?;
                break;
            }
        }
        let body = parse_block(cursor, Scope::Function)?;
        Ok(Self {
            line,
            name,
            is_public,
            is_static,
            is_default,
            is_override,
            type_params,
            params,
            body,
        })
    }
}
