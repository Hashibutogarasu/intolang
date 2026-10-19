use ast::Expr;
use types::{Literal, Type};

use native::Operand;

use crate::context::Lowerer;

impl Lowerer<'_> {
    /// The right-hand side of a numeric operation: a number literal, or a
    /// native numeric variable.
    pub fn numeric_operand(&self, expression: &Expr) -> Option<Operand> {
        match expression {
            Expr::Literal(Literal::Int(number)) => Some(Operand::Number(number.to_string())),
            Expr::Literal(Literal::Float(number)) => Some(Operand::Number(number.to_string())),
            Expr::Literal(Literal::Char(value)) => {
                Some(Operand::Number(u32::from(*value).to_string()))
            }
            other => {
                let name = other.as_ident()?;
                let variable = self.state.values.get(name)?;
                (variable.native && (variable.ty.is_numeric() || variable.ty == Type::Char))
                    .then(|| Operand::Slot(name.to_owned()))
            }
        }
    }
}
