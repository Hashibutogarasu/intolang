use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};

/// `break;` inside a `while` loop.
pub struct BreakStatement;

impl Format for BreakStatement {
    fn format(&self, line: &str) -> Option<String> {
        if line == "break;" || line == "break" {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for BreakStatement {
    fn parse(&self, ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(line) = cursor.line() else {
            return Ok(false);
        };
        if line.code != "break;" && line.code != "break" {
            return Ok(false);
        }
        if !ctx.blocks.contains(&'w') {
            return Err(ctx.diagnostic(line.number, "`break` must be inside a while loop"));
        }
        ctx.output.push('\u{1e}');
        ctx.output.push_str("NATIVE_BREAK\n");
        cursor.advance(1);
        Ok(true)
    }
}
