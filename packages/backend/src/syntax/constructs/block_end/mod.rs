use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};

/// A closing `}` for an `init`, `if` or `while` block.
pub struct BlockEnd;

impl Format for BlockEnd {
    fn format(&self, line: &str) -> Option<String> {
        if line == "}" {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for BlockEnd {
    fn parse(&self, ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(line) = cursor.line() else {
            return Ok(false);
        };
        if line.code != "}" {
            return Ok(false);
        }
        match ctx.blocks.pop() {
            Some('w') => {
                ctx.output.push('\u{1e}');
                ctx.output.push_str("NATIVE_LOOP_END\n");
            }
            Some('f') => {
                ctx.output.push('\u{1e}');
                ctx.output.push_str("NATIVE_IF_END\n");
            }
            Some('i') => ctx.in_init = false,
            _ => {
                return Err(ctx.diagnostic(line.number, "Unexpected closing brace"));
            }
        }
        cursor.advance(1);
        Ok(true)
    }
}
