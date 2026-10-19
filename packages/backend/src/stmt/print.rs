use anyhow::Result;
use ast::Expr;
use types::Literal;

use crate::context::Lowerer;
use crate::diagnostic::diagnostic;
use crate::emit;
use crate::expr::base_type_name;

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
            if (path.segments().len() > 2
                || self.is_type_property(path)
                || self.is_dynamic_member(path)
                || (path.is_member()
                    && self
                        .type_of(argument)
                        .is_ok_and(|ty| self.is_instance_type(&ty))))
                && self.state.values.get(path.first()).is_some()
            {
                let Some((slot, ty)) = self
                    .lower_field_read(path)
                    .map_err(|message| self.diagnostic(line, &message))?
                else {
                    return Err(self.diagnostic(line, &format!("Undefined variable `{argument}`")));
                };
                if self.is_instance_type(&ty) {
                    return self.print_instance(line, &slot, newline);
                }
                emit::print(&mut self.state.output, &slot, &ty, newline);
                return Ok(());
            }
            if path.is_member()
                && let Some(variable) = self.state.values.get(path.first()).cloned()
            {
                self.ensure_usable(path.first())
                    .map_err(|message| self.diagnostic(line, &message))?;
                let member = path.segments()[1..].join(".");
                let class = base_type_name(&variable.value).to_owned();
                let fields = env.class_fields.get(&class).ok_or_else(|| {
                    self.diagnostic(
                        line,
                        &format!("Type `{class}` has no declared member fields"),
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
                    &class,
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
            if variable.native && self.is_instance_type(&variable.ty) {
                return self.print_instance(line, name, newline);
            }
            return self.print_variable(line, name, &variable, newline);
        }
        if let Expr::Literal(literal) = argument {
            self.print_literal(literal, newline);
            return Ok(());
        }
        if matches!(
            argument,
            Expr::Call { .. }
                | Expr::Binary { .. }
                | Expr::Cast { .. }
                | Expr::Compare { .. }
                | Expr::Logical { .. }
                | Expr::Not(_)
        ) {
            let temporary = format!("__print{}", self.state.temp_counter);
            self.state.temp_counter += 1;
            let (value, ty, native) = self
                .declaration_value(argument, None, &env.type_params, &temporary)
                .map_err(|message| self.diagnostic(line, &message))?;
            if native && self.is_instance_type(&ty) {
                self.state
                    .values
                    .insert(temporary.clone(), crate::context::Variable::temporary(ty));
                return self.print_instance(line, &temporary, newline);
            }
            if native {
                emit::print(&mut self.state.output, &temporary, &ty, newline);
            } else {
                emit::text(&mut self.state.output, &value, newline);
            }
            return Ok(());
        }
        Err(self.diagnostic(line, &format!("Undefined variable `{argument}`")))
    }

    /// Whether values of the type are instances of a declared class, which
    /// print through their `toString`.
    fn is_instance_type(&self, ty: &types::Type) -> bool {
        ty.referent().class_name().is_some_and(|name| {
            self.env.class_fields.contains_key(name)
                && !matches!(name, "String" | "List" | "Callback" | "BiCallback")
        })
    }

    /// Print the text that `toString` of the instance in `slot` gives.
    fn print_instance(&mut self, line: usize, slot: &str, newline: bool) -> Result<()> {
        let env = self.env;
        let call = Expr::Call {
            callee: ast::Path::from(vec![slot.to_owned(), "toString".to_owned()]),
            args: Vec::new(),
        };
        let temporary = format!("__print{}", self.state.temp_counter);
        self.state.temp_counter += 1;
        let (value, ty, native) = self
            .declaration_value(&call, None, &env.type_params, &temporary)
            .map_err(|message| self.diagnostic(line, &message))?;
        if native {
            emit::print(&mut self.state.output, &temporary, &ty, newline);
        } else {
            emit::text(&mut self.state.output, &value, newline);
        }
        Ok(())
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
