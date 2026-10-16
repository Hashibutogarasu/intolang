use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};

/// The `} else {` continuation of an `if` block.
pub struct ElseBlock;

impl Format for ElseBlock {
    fn format(&self, line: &str) -> Option<String> {
        if line == "} else {" {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for ElseBlock {
    fn parse(&self, ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(line) = cursor.line() else {
            return Ok(false);
        };
        if line.code != "} else {" {
            return Ok(false);
        }
        if ctx.blocks.last() != Some(&'f') {
            return Err(ctx.diagnostic(line.number, "`else` must follow an `if` block"));
        }
        ctx.output.push('\u{1e}');
        ctx.output.push_str("NATIVE_IF_ELSE\n");
        cursor.advance(1);
        Ok(true)
    }
}
