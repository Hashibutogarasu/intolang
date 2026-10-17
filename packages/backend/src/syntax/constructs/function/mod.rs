use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};
use crate::syntax::support::text::{brace_delta, strip_keyword};

/// A `pub fun name(...) { ... }` declaration.
///
/// Function signatures and bodies are collected during preparation so calls
/// can resolve forward references; this construct consumes the declaration.
pub struct FunctionDecl;

/// Whether a line opens a function declaration, ignoring `pub`/`static`.
fn is_function_header(line: &str) -> bool {
    let line = strip_keyword(line, "pub").unwrap_or(line);
    let line = strip_keyword(line, "static").unwrap_or(line);
    strip_keyword(line, "fun").is_some()
}

impl Format for FunctionDecl {
    fn format(&self, line: &str) -> Option<String> {
        if is_function_header(line) && line.contains('{') {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for FunctionDecl {
    fn parse(&self, _ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(first) = cursor.line() else {
            return Ok(false);
        };
        if !is_function_header(&first.code) || !first.code.contains('{') {
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
