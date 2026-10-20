use errors::Result;
use types::Punct;

use super::statement::Stmt;
use crate::Cursor;

/// Where a block appears; decides which nested declarations are errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    Module,
    Block,
    Function,
}

/// Parse `{ statement* }`.
pub(crate) fn parse_block(cursor: &mut Cursor<'_>, scope: Scope) -> Result<Vec<Stmt>> {
    cursor.expect_punct(Punct::LeftBrace, "`{`")?;
    let mut body = Vec::new();
    loop {
        if cursor.eat_punct(Punct::RightBrace) {
            return Ok(body);
        }
        if cursor.at_end() {
            return Err(cursor.error_expecting("`}`", "Expected `}` to close the block"));
        }
        let start = cursor.position();
        match Stmt::parse(cursor, scope) {
            Ok(stmt) => body.push(stmt),
            Err(error) => cursor.recover(error, start)?,
        }
    }
}
