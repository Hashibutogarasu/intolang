use errors::Result;
use types::{Keyword, Punct};

use super::attribute::Attribute;
use crate::Cursor;

/// `pub macro name { #[attribute] }`: declares attributes other code can use.
#[derive(Debug, Clone, PartialEq)]
pub struct MacroDecl {
    pub line: usize,
    pub name: String,
    pub is_public: bool,
    pub attributes: Vec<Attribute>,
}

impl MacroDecl {
    pub(crate) fn ahead(cursor: &Cursor<'_>) -> bool {
        let offset = usize::from(cursor.peek_keyword(0) == Some(Keyword::Pub));
        cursor.peek_keyword(offset) == Some(Keyword::Macro)
            && cursor.peek_word(offset + 1).is_some()
    }
}

impl TryFrom<&mut Cursor<'_>> for MacroDecl {
    type Error = errors::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        let line = cursor.line();
        let is_public = cursor.eat_keyword(Keyword::Pub);
        cursor.eat_keyword(Keyword::Macro);
        let (name, _) = cursor.expect_ident("a macro name")?;
        cursor.expect_punct(Punct::LeftBrace, "`{` after the macro name")?;
        let attributes = Attribute::parse_all(cursor)?;
        cursor.expect_punct(Punct::RightBrace, "`}` to close the macro")?;
        Ok(Self {
            line,
            name,
            is_public,
            attributes,
        })
    }
}
