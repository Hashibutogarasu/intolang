use ast::Expr;
use types::{Literal, Type};

use crate::context::Lowerer;
use crate::emit;

impl Lowerer<'_> {
    /// Print a literal value.
    pub fn print_literal(&mut self, literal: &Literal, newline: bool) {
        let output = &mut self.state.output;
        match literal {
            Literal::String(value) => emit::text(output, value, newline),
            Literal::Int(number) => emit::print_const(output, &Type::Int32, &number.to_string(), newline),
            Literal::Float(number) => {
                emit::print_const(output, &Type::Float64, &number.to_string(), newline)
            }
            Literal::Bool(flag) => {
                emit::print_const(output, &Type::Boolean, if *flag { "1" } else { "0" }, newline)
            }
        }
    }

    /// Print an argument whose value is known without running anything: a
    /// static class field with a literal default, an instance field of a
    /// bound value, or a string literal. Returns whether it printed.
    pub fn print_compile_time(&mut self, argument: &Expr, newline: bool) -> bool {
        let env = self.env;
        if let Expr::Path(path) = argument {
            if !self.state.values.contains_key(path.first()) {
                if let Some(literal) = self.static_field(path).and_then(|field| field.default_literal()) {
                    self.print_literal(literal, newline);
                    return true;
                }
            }
            if path.is_member() {
                let field = path.segments()[1..].join(".");
                let variable = self.state.values.get(path.first()).cloned();
                let found = variable.as_ref().and_then(|variable| {
                    let fields = env.class_fields.get(&variable.value)?;
                    let (index, def) = fields
                        .iter()
                        .filter(|def| !def.is_static)
                        .enumerate()
                        .find(|(_, def)| def.name == field)?;
                    Some((variable.value.clone(), index, def))
                });
                if let Some((class, index, def)) = found {
                    emit::field_print(&mut self.state.output, path.first(), &class, index, &def.ty, newline);
                    return true;
                }
            }
        }
        if let Expr::Literal(Literal::String(text)) = argument {
            emit::text(&mut self.state.output, text, newline);
            return true;
        }
        false
    }
}
