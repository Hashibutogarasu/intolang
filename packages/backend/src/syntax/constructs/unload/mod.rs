use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};

/// `unload name;` — marks a memory backed binding as unloaded.
pub struct Unload;

impl Format for Unload {
    fn format(&self, line: &str) -> Option<String> {
        if line.starts_with("unload ") {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for Unload {
    fn parse(&self, ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(line) = cursor.line() else {
            return Ok(false);
        };
        let Some(rest) = line.code.strip_prefix("unload ") else {
            return Ok(false);
        };
        let name = rest.trim_end_matches(';').trim();
        if let Some(variable) = ctx.values.get_mut(name) {
            variable.allocated = false;
            variable.unloaded = true;
        }
        cursor.advance(1);
        Ok(true)
    }
}
