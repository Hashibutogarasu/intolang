use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};
use crate::syntax::support::{
    diagnostic, emit_link_call, inline_user_function, parse_call, resolve_receiver,
};

/// A method call statement such as `client.send("hi");`.
pub struct MethodCall;

impl Format for MethodCall {
    fn format(&self, line: &str) -> Option<String> {
        let call = line.trim_end_matches(';');
        if call.contains('.') && call.contains('(') && call.ends_with(')') {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for MethodCall {
    fn parse(&self, ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(line) = cursor.line() else {
            return Ok(false);
        };
        let Some((function, args)) = parse_call(line.code.trim_end_matches(';')) else {
            return Ok(false);
        };
        if !function.contains('.') {
            return Ok(false);
        }
        let path = ctx.source_path.clone();
        let scope = ctx.scope_name();
        let (receiver, receiver_type) = resolve_receiver(
            &mut ctx.output,
            function,
            &mut ctx.values,
            &ctx.native_links,
            &ctx.enum_values,
            &mut ctx.temp_counter,
        )
        .map_err(|message| diagnostic(&path, line.number, scope, &message))?;
        let method = function
            .split('.')
            .filter(|segment| !segment.is_empty())
            .next_back()
            .unwrap_or_default();
        let key = format!("{}.{}", receiver_type.name(), method);
        if let Some(inner) = ctx.user_functions.get(&key).cloned() {
            let _ = inline_user_function(
                &mut ctx.output,
                &inner,
                Some(receiver),
                args,
                &mut ctx.values,
                &ctx.native_links,
                &ctx.enum_values,
                &mut ctx.temp_counter,
                &ctx.user_functions,
                0,
            )
            .map_err(|message| diagnostic(&path, line.number, scope, &message))?;
            cursor.advance(1);
            return Ok(true);
        }
        let link = ctx.native_links.get(&key).ok_or_else(|| {
            diagnostic(
                &path,
                line.number,
                scope,
                &format!("Undefined method `{key}`"),
            )
        })?;
        let combined = if args.trim().is_empty() {
            receiver
        } else {
            format!("{receiver},{args}")
        };
        emit_link_call(
            &mut ctx.output,
            link,
            &combined,
            &ctx.enum_values,
            &ctx.values,
            "",
            true,
        )
        .map_err(|message| diagnostic(&path, line.number, scope, &message))?;
        cursor.advance(1);
        Ok(true)
    }
}
