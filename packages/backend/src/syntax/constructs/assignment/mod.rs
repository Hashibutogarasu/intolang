use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};
use crate::syntax::support::{declaration_value, diagnostic, is_identifier};
use crate::types::Type;

/// Reassignment of an existing binding: `name = value;`.
pub struct Assignment;

impl Format for Assignment {
    fn format(&self, line: &str) -> Option<String> {
        let declaration = line.trim_end_matches(';');
        let (lhs, rhs) = declaration.split_once('=')?;
        if is_identifier(lhs.trim()) && !rhs.trim_start().starts_with('=') {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for Assignment {
    fn parse(&self, ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(line) = cursor.line() else {
            return Ok(false);
        };
        let declaration = line.code.trim_end_matches(';');
        let Some((lhs, rhs)) = declaration.split_once('=') else {
            return Ok(false);
        };
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        if !is_identifier(lhs) || !ctx.values.contains_key(lhs) || rhs.starts_with('=') {
            return Ok(false);
        }
        let target_ty = ctx
            .values
            .get(lhs)
            .map(|value| value.ty.clone())
            .unwrap_or(Type::Void);
        let path = ctx.source_path.clone();
        let scope = ctx.scope_name();
        let (value, ty, native_value) = declaration_value(
            rhs,
            Some(target_ty),
            &mut ctx.values,
            &ctx.imported_functions,
            &ctx.native_links,
            &ctx.enum_values,
            &ctx.type_params,
            &mut ctx.output,
            lhs,
            &mut ctx.temp_counter,
            &ctx.user_functions,
        )
        .map_err(|message| diagnostic(&path, line.number, scope, &message))?;
        if let Some(variable) = ctx.values.get_mut(lhs) {
            variable.value = value;
            variable.ty = ty;
            variable.native = native_value;
            variable.unloaded = false;
            variable.allocated = true;
        }
        cursor.advance(1);
        Ok(true)
    }
}
