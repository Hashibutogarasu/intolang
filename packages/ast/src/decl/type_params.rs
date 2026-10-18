use anyhow::Result;
use types::Punct;

use crate::Cursor;

/// Parse an optional `<A, B>` list that follows a declaration name.
pub(super) fn parse_type_params(cursor: &mut Cursor<'_>) -> Result<Vec<String>> {
    let mut type_params = Vec::new();
    if cursor.peek_symbol(0) != Some('<') {
        return Ok(type_params);
    }
    cursor.bump();
    loop {
        let (param, _) = cursor.expect_ident("a type parameter name")?;
        type_params.push(param);
        if !cursor.eat_punct(Punct::Comma) {
            break;
        }
    }
    if cursor.peek_symbol(0) != Some('>') {
        return Err(cursor.error("Expected `>` after the type parameters"));
    }
    cursor.bump();
    Ok(type_params)
}
