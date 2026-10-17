use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};
use crate::syntax::support::text::{brace_delta, is_identifier};

/// A `Name() { ... }` data carrier declaration.
///
/// Class fields are collected during preparation (see `support::analysis`);
/// this construct consumes the declaration so its body is not parsed as
/// statements.
pub struct Class;

impl Format for Class {
    fn format(&self, line: &str) -> Option<String> {
        let (prefix, _) = line.split_once("() {")?;
        let name = prefix.strip_prefix("pub ").unwrap_or(prefix).trim();
        if is_identifier(name) {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for Class {
    fn parse(&self, _ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(first) = cursor.line() else {
            return Ok(false);
        };
        let Some((prefix, _)) = first.code.split_once("() {") else {
            return Ok(false);
        };
        let name = prefix.strip_prefix("pub ").unwrap_or(prefix).trim();
        if !is_identifier(name) {
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
