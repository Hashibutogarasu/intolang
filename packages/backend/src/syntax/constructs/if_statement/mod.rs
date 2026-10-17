use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};
use crate::syntax::support::{
    comparison_operand, diagnostic, emit_comparison, emit_link_call, parse_call, parse_comparison,
    resolve_method,
};

/// An `if condition { ... }` block.
pub struct IfStatement;

impl Format for IfStatement {
    fn format(&self, line: &str) -> Option<String> {
        if line.starts_with("if ") && line.ends_with('{') {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for IfStatement {
    fn parse(&self, ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(line) = cursor.line() else {
            return Ok(false);
        };
        let Some(condition) = line
            .code
            .strip_prefix("if ")
            .and_then(|value| value.strip_suffix('{'))
            .map(str::trim)
        else {
            return Ok(false);
        };
        let condition = condition
            .strip_prefix('(')
            .and_then(|value| value.strip_suffix(')'))
            .unwrap_or(condition)
            .trim();
        let path = ctx.source_path.clone();
        let scope = ctx.scope_name();
        if let Some((left, operator, right)) = parse_comparison(condition) {
            let ParseContext { values, output, .. } = ctx;
            let left_var = values
                .get(left)
                .filter(|value| value.native && value.ty.is_numeric())
                .ok_or_else(|| {
                    diagnostic(
                        &path,
                        line.number,
                        scope,
                        "if condition requires an integer or float value",
                    )
                })?;
            let operand = comparison_operand(right, values).ok_or_else(|| {
                diagnostic(
                    &path,
                    line.number,
                    scope,
                    "if condition requires a numeric literal or value",
                )
            })?;
            let ty = left_var.ty.clone();
            emit_comparison(output, "NATIVE_IF_BEGIN", left, operator, &operand, &ty);
        } else {
            let Some((function, args)) = parse_call(condition) else {
                return Err(diagnostic(
                    &path,
                    line.number,
                    scope,
                    "if condition must compare numeric values or call a Boolean function",
                ));
            };
            let (link, combined, has_receiver) = if function.contains('.') {
                let (link, combined) = resolve_method(
                    &mut ctx.output,
                    function,
                    args,
                    &mut ctx.values,
                    &ctx.native_links,
                    &ctx.enum_values,
                    &mut ctx.temp_counter,
                )
                .map_err(|message| diagnostic(&path, line.number, scope, &message))?;
                (link, combined, true)
            } else {
                if !ctx.imported_functions.contains(function) {
                    return Err(diagnostic(
                        &path,
                        line.number,
                        scope,
                        &format!("Function `{function}` is not imported"),
                    ));
                }
                let link = ctx.native_links.get(function).ok_or_else(|| {
                    diagnostic(
                        &path,
                        line.number,
                        scope,
                        &format!("Function `{function}` has no native link"),
                    )
                })?;
                (link.clone(), args.to_owned(), false)
            };
            if link.return_type != "Boolean" {
                return Err(diagnostic(
                    &path,
                    line.number,
                    scope,
                    &format!("if condition `{function}` must return Boolean"),
                ));
            }
            let marker_start = ctx.output.len();
            emit_link_call(
                &mut ctx.output,
                &link,
                &combined,
                &ctx.enum_values,
                &ctx.values,
                "",
                has_receiver,
            )
            .map_err(|message| diagnostic(&path, line.number, scope, &message))?;
            let marker =
                ctx.output[marker_start..].replacen("NATIVE_LINK_ASSIGN", "NATIVE_IF_CALL", 1);
            ctx.output.replace_range(marker_start.., &marker);
        }
        ctx.blocks.push('f');
        cursor.advance(1);
        Ok(true)
    }
}
