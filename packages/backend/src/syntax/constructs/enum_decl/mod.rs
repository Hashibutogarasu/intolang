use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};
use crate::syntax::support::text::brace_delta;

/// An `enum Name { ... }` declaration (values are collected during preparation).
pub struct EnumDecl;

impl Format for EnumDecl {
    fn format(&self, line: &str) -> Option<String> {
        if line.starts_with("enum ") {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for EnumDecl {
    fn parse(&self, _ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(first) = cursor.line() else {
            return Ok(false);
        };
        if !first.code.starts_with("enum ") {
            return Ok(false);
        }
        let mut consumed = 1;
        let mut depth = brace_delta(&first.code);
        while depth > 0 {
            let Some(next) = cursor.peek(consumed) else {
                break;
            };
            depth += brace_delta(&next.code);
            consumed += 1;
        }
        cursor.advance(consumed);
        Ok(true)
    }
}
