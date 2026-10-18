use anyhow::Result;
use types::Keyword;

use crate::decl::{Attribute, ClassDecl, EnumDecl, FunctionDecl, MacroDecl, UseDecl};
use crate::{Cursor, Stmt, SyntaxError};

/// One top-level element of a module.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Import(UseDecl),
    Export(UseDecl),
    Enum(EnumDecl),
    Class(ClassDecl),
    Function(FunctionDecl),
    Macro(MacroDecl),
    Stmt(Stmt),
}

impl TryFrom<&mut Cursor<'_>> for Item {
    type Error = anyhow::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        if ClassDecl::ahead(cursor) {
            return Ok(Item::Class(ClassDecl::try_from(&mut *cursor)?));
        }
        if FunctionDecl::ahead(cursor) {
            return Ok(Item::Function(FunctionDecl::try_from(&mut *cursor)?));
        }
        if let Some(attribute) = Attribute::parse_all(&mut cursor.clone())?.first() {
            return Err(SyntaxError::new(
                attribute.line,
                "An attribute must be followed by a function or class",
            )
            .into());
        }
        if UseDecl::ahead(cursor, Keyword::Import) {
            return Ok(Item::Import(UseDecl::parse(cursor, Keyword::Import)?));
        }
        if UseDecl::ahead(cursor, Keyword::Export) {
            return Ok(Item::Export(UseDecl::parse(cursor, Keyword::Export)?));
        }
        if EnumDecl::ahead(cursor) {
            return Ok(Item::Enum(EnumDecl::try_from(&mut *cursor)?));
        }
        if MacroDecl::ahead(cursor) {
            return Ok(Item::Macro(MacroDecl::try_from(&mut *cursor)?));
        }
        Ok(Item::Stmt(Stmt::try_from(&mut *cursor)?))
    }
}
