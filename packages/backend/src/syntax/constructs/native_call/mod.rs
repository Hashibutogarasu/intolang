use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};
use crate::syntax::support::{diagnostic, imported_call};

/// A single argument call to an imported native function that is lowered to a
/// plain `NATIVE` message (e.g. `puts("hi");`).
pub struct NativeCall;

impl Format for NativeCall {
    fn format(&self, line: &str) -> Option<String> {
        if line.ends_with(';') && line.contains('(') {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for NativeCall {
    fn parse(&self, ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(line) = cursor.line() else {
            return Ok(false);
        };
        let Some((symbol, arg)) = imported_call(&line.code, &ctx.imported_functions) else {
            return Ok(false);
        };
        let value = arg.trim();
        let (kind, value) = if value.starts_with('"') && value.ends_with('"') {
            ("s", value[1..value.len() - 1].to_owned())
        } else if value.parse::<i32>().is_ok() {
            ("i", value.to_owned())
        } else if let Some((enum_name, variant)) = value.split_once('.') {
            let discriminant = ctx
                .enum_values
                .get(enum_name)
                .and_then(|variants| variants.get(variant))
                .ok_or_else(|| {
                    ctx.diagnostic(line.number, &format!("Undefined enum variant `{value}`"))
                })?;
            ("i", discriminant.to_string())
        } else if let Some(variable) = ctx.values.get(value) {
            ("s", variable.value.clone())
        } else {
            return Err(diagnostic(
                &ctx.source_path,
                line.number,
                ctx.scope_name(),
                &format!("Native call `{symbol}` expects a string or 32-bit integer argument"),
            ));
        };
        ctx.output.push('\u{1e}');
        ctx.output.push_str("NATIVE\t");
        ctx.output.push_str(symbol);
        ctx.output.push('\t');
        ctx.output.push_str(kind);
        ctx.output.push('\t');
        if kind == "i" {
            ctx.output.push_str(&value);
        } else {
            for byte in value.bytes() {
                ctx.output.push_str(&format!("{byte:02x}"));
            }
        }
        ctx.output.push('\n');
        cursor.advance(1);
        Ok(true)
    }
}
