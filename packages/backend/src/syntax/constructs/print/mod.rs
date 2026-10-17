use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::parse::{Cursor, Parse};
use crate::syntax::support::{Literal, diagnostic, emit_print, emit_print_const, parse_literal};
use crate::types::Type;

/// A `print`/`println` statement, including member, literal and variable forms.
pub struct Print;

impl Format for Print {
    fn format(&self, line: &str) -> Option<String> {
        if line.starts_with("print(") || line.starts_with("println(") {
            Some(line.to_owned())
        } else {
            None
        }
    }
}

impl Parse for Print {
    fn parse(&self, ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool> {
        let Some(line) = cursor.line() else {
            return Ok(false);
        };
        let Some((is_println, rest)) = line
            .code
            .strip_prefix("println(")
            .map(|rest| (true, rest))
            .or_else(|| line.code.strip_prefix("print(").map(|rest| (false, rest)))
        else {
            return Ok(false);
        };
        let path = ctx.source_path.clone();
        let scope = ctx.scope_name();
        let function = if is_println { "println" } else { "print" };
        if !ctx.imported_functions.contains(function) {
            return Err(diagnostic(
                &path,
                line.number,
                scope,
                &format!("Function `{function}` is not imported"),
            ));
        }
        let arg = rest.split(')').next().unwrap_or("").trim();
        if let Some((base, field)) = arg.split_once('.') {
            if let Some(variable) = ctx.values.get(base) {
                let fields = ctx.class_fields.get(&variable.value).ok_or_else(|| {
                    diagnostic(
                        &path,
                        line.number,
                        scope,
                        &format!("Type `{}` has no declared member fields", variable.value),
                    )
                })?;
                let (index, field_def) = fields
                    .iter()
                    .enumerate()
                    .find(|(_, field_def)| field_def.name == field)
                    .ok_or_else(|| {
                        diagnostic(
                            &path,
                            line.number,
                            scope,
                            &format!("Undefined member `{field}` on `{}`", variable.value),
                        )
                    })?;
                ctx.output.push('\u{1e}');
                ctx.output.push_str("NATIVE_FIELD_PRINT\t");
                ctx.output.push_str(base);
                ctx.output.push('\t');
                ctx.output.push_str(&variable.value);
                ctx.output.push('\t');
                ctx.output.push_str(&index.to_string());
                ctx.output.push('\t');
                ctx.output.push_str(&field_def.ty);
                ctx.output.push('\t');
                ctx.output.push_str(if is_println { "1" } else { "0" });
                ctx.output.push('\n');
                cursor.advance(1);
                return Ok(true);
            }
        }
        if arg.starts_with('"') && arg.ends_with('"') {
            ctx.output.push_str(&arg[1..arg.len() - 1]);
            if is_println {
                ctx.output.push('\n');
            }
            cursor.advance(1);
            return Ok(true);
        }
        if let Some(variable) = ctx.values.get(arg).cloned() {
            if (!variable.memory_backed && !ctx.in_init)
                || (variable.declared_in_init && !ctx.in_init)
            {
                let scope_label = if ctx.init_has_run {
                    "module scope (after init)"
                } else {
                    "module scope"
                };
                return Err(diagnostic(
                    &path,
                    line.number,
                    scope_label,
                    &format!("Variable `{arg}` is out of scope here"),
                ));
            }
            let allocated = if variable.memory_backed
                && !variable.allocated
                && variable.lazy
                && !variable.unloaded
            {
                true
            } else {
                variable.allocated
            };
            if variable.memory_backed && !allocated {
                return Err(diagnostic(
                    &path,
                    line.number,
                    scope,
                    &format!("Variable `{arg}` was unloaded"),
                ));
            }
            if allocated && !variable.allocated {
                if let Some(stored) = ctx.values.get_mut(arg) {
                    stored.allocated = true;
                }
            }
            if variable.native {
                emit_print(&mut ctx.output, arg, &variable.ty, is_println);
            } else {
                ctx.output.push_str(&variable.value);
                if is_println {
                    ctx.output.push('\n');
                }
            }
            cursor.advance(1);
            return Ok(true);
        }
        if let Some(literal) = parse_literal(arg) {
            match literal {
                Literal::Int(number) => emit_print_const(
                    &mut ctx.output,
                    &Type::Int32,
                    &number.to_string(),
                    is_println,
                ),
                Literal::Float(number) => emit_print_const(
                    &mut ctx.output,
                    &Type::Float64,
                    &number.to_string(),
                    is_println,
                ),
                Literal::Bool(flag) => emit_print_const(
                    &mut ctx.output,
                    &Type::Boolean,
                    if flag { "1" } else { "0" },
                    is_println,
                ),
                Literal::String(value) => {
                    ctx.output.push_str(&value);
                    if is_println {
                        ctx.output.push('\n');
                    }
                }
            }
            cursor.advance(1);
            return Ok(true);
        }
        Err(diagnostic(
            &path,
            line.number,
            scope,
            &format!("Undefined variable `{arg}`"),
        ))
    }
}
