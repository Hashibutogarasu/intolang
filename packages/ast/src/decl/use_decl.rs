use anyhow::Result;
use types::Keyword;

use crate::Cursor;

/// The URI of an `import "scheme://module";` or `export "alias://path";`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseDecl {
    pub line: usize,
    pub uri: String,
}

impl UseDecl {
    pub(crate) fn ahead(cursor: &Cursor<'_>, keyword: Keyword) -> bool {
        cursor.peek_keyword(0) == Some(keyword) && cursor.peek_string(1).is_some()
    }

    pub(crate) fn parse(cursor: &mut Cursor<'_>, keyword: Keyword) -> Result<Self> {
        let line = cursor.line();
        if !cursor.eat_keyword(keyword) {
            return Err(cursor.error("Expected a module declaration"));
        }
        let Some(uri) = cursor.peek_string(0).map(str::to_owned) else {
            return Err(cursor.error("Expected a module URI string"));
        };
        cursor.bump();
        cursor.end_statement()?;
        Ok(Self { line, uri })
    }
}
