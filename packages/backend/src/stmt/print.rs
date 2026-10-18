use anyhow::Result;
use ast::Expr;
use types::Literal;

use crate::context::Lowerer;
use crate::diagnostic::diagnostic;
use crate::emit;

impl Lowerer<'_> {
    /// Lower `print(value)` / `println(value)`: a static field constant, an
    /// instance field, a string or number literal, or a bound variable.
    pub fn lower_print(&mut self, line: usize, newline: bool, args: &[Expr]) -> Result<()> {
        let env = self.env;
        let function = if newline { "println" } else { "print" };
        if !env.imported_functions.contains(function) {
            return Err(self.diagnostic(line, &format!("Function `{function}` is not imported")));
        }
        let [argument] = args else {
            let text = args.iter().map(ToString::to_string).collect::<Vec<_>>();
            return Err(self.diagnostic(line, &format!("Undefined variable `{}`", text.join(", "))));
        };
        if let Expr::Path(path) = argument {
            if !self.state.values.contains_key(path.first())
                && let Some(field) = self.static_field(path)
            {
                let Some(literal) = field.default_literal() else {
                    return Err(self.diagnostic(
                        line,
                        &format!("Static field `{argument}` has no constant default"),
                    ));
                };
                self.print_literal(literal, newline);
                return Ok(());
            }
            if path.is_member()
                && let Some(variable) = self.state.values.get(path.first()).cloned()
            {
                self.ensure_usable(path.first())
                    .map_err(|message| self.diagnostic(line, &message))?;
                let member = path.segments()[1..].join(".");
                let fields = env.class_fields.get(&variable.value).ok_or_else(|| {
                    self.diagnostic(
                        line,
                        &format!("Type `{}` has no declared member fields", variable.value),
                    )
                })?;
                let (index, field) = fields
                    .iter()
                    .filter(|field| !field.is_static)
                    .enumerate()
                    .find(|(_, field)| field.name == member)
                    .ok_or_else(|| {
                        self.diagnostic(
                            line,
                            &format!("Undefined member `{member}` on `{}`", variable.value),
                        )
                    })?;
                emit::field_print(
                    &mut self.state.output,
                    path.first(),
                    &variable.value,
                    index,
                    &field.ty,
                    newline,
                );
                return Ok(());
            }
        }
        if let Expr::Literal(Literal::String(text)) = argument {
            emit::text(&mut self.state.output, text, newline);
            return Ok(());
        }
        if let Some(name) = argument.as_ident()
            && let Some(variable) = self.state.values.get(name).cloned()
        {
            self.ensure_usable(name)
                .map_err(|message| self.diagnostic(line, &message))?;
            return self.print_variable(line, name, &variable, newline);
        }
        if let Expr::Literal(literal) = argument {
            self.print_literal(literal, newline);
            return Ok(());
        }
        Err(self.diagnostic(line, &format!("Undefined variable `{argument}`")))
    }

    fn print_variable(
        &mut self,
        line: usize,
        name: &str,
        variable: &crate::context::Variable,
        newline: bool,
    ) -> Result<()> {
        let in_init = self.state.in_init;
        if (!variable.memory_backed && !in_init) || (variable.declared_in_init && !in_init) {
            let scope = if self.state.init_has_run {
                "module scope (after init)"
            } else {
                "module scope"
            };
            return Err(diagnostic(
                &self.env.source_path,
                line,
                scope,
                &format!("Variable `{name}` is out of scope here"),
            ));
        }
        let allocated =
            (variable.memory_backed && !variable.allocated && variable.lazy && !variable.unloaded)
                || variable.allocated;
        if variable.memory_backed && !allocated {
            return Err(self.diagnostic(line, &format!("Variable `{name}` was unloaded")));
        }
        if allocated
            && !variable.allocated
            && let Some(stored) = self.state.values.get_mut(name)
        {
            stored.allocated = true;
        }
        if variable.native {
            emit::print(&mut self.state.output, name, &variable.ty, newline);
        } else {
            emit::text(&mut self.state.output, &variable.value, newline);
        }
        Ok(())
    }
}
