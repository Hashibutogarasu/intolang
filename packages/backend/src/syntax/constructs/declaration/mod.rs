use anyhow::Result;

use crate::syntax::context::{ParseContext, Variable};
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};
use crate::syntax::support::{declaration_value, diagnostic, split_annotation};

/// A binding declaration: `into`, `now into`, `late into` or `const now into`.
pub struct Declaration;

impl Format for Declaration {
    fn format(&self, line: &str) -> Option<String> {
        let declaration = line.trim_end_matches(';');
        let prefixes = ["const now into ", "now into ", "late into ", "into "];
        if prefixes
            .iter()
            .any(|prefix| declaration.starts_with(prefix))
        {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for Declaration {
    fn parse(&self, ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(line) = cursor.line() else {
            return Ok(false);
        };
        let declaration = line.code.trim_end_matches(';');
        let (prefix, decl) = if let Some(rest) = declaration.strip_prefix("const now into ") {
            ("now", rest)
        } else if let Some(rest) = declaration.strip_prefix("now into ") {
            ("now", rest)
        } else if let Some(rest) = declaration.strip_prefix("late into ") {
            ("late", rest)
        } else if let Some(rest) = declaration.strip_prefix("into ") {
            ("local", rest)
        } else {
            return Ok(false);
        };
        let Some((lhs, rhs)) = decl.split_once('=') else {
            cursor.advance(1);
            return Ok(true);
        };
        let (name, annotation) = split_annotation(lhs.trim(), &ctx.type_params)
            .map_err(|error| ctx.diagnostic(line.number, &error.to_string()))?;
        if name.is_empty() {
            return Err(ctx.diagnostic(line.number, "Expected a variable name after `into`"));
        }
        let memory = prefix != "local";
        let path = ctx.source_path.clone();
        let scope = ctx.scope_name();
        let (value, ty, native_value) = declaration_value(
            rhs.trim(),
            annotation,
            &mut ctx.values,
            &ctx.imported_functions,
            &ctx.native_links,
            &ctx.enum_values,
            &ctx.type_params,
            &mut ctx.output,
            &name,
            &mut ctx.temp_counter,
            &ctx.user_functions,
        )
        .map_err(|message| diagnostic(&path, line.number, scope, &message))?;
        ctx.values.insert(
            name,
            Variable {
                value,
                ty,
                memory_backed: memory,
                allocated: prefix == "now",
                lazy: prefix == "late",
                unloaded: false,
                declared_in_init: ctx.in_init,
                native: native_value,
            },
        );
        cursor.advance(1);
        Ok(true)
    }
}
