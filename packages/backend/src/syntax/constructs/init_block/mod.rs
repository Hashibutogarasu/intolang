use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};

/// The `init { ... }` entry block.
pub struct InitBlock;

impl Format for InitBlock {
    fn format(&self, line: &str) -> Option<String> {
        if line.starts_with("init") && line.contains('{') {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for InitBlock {
    fn parse(&self, ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(line) = cursor.line() else {
            return Ok(false);
        };
        if line.code.starts_with("init") && line.code.contains('{') {
            ctx.in_init = true;
            ctx.init_has_run = true;
            ctx.blocks.push('i');
            cursor.advance(1);
            return Ok(true);
        }
        Ok(false)
    }
}
