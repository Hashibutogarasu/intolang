use ast::Expr;
use native::{Comparison, Operand};
use types::{ComparisonOperator, Literal, MessageTag, Type};

use crate::context::Lowerer;
use crate::emit;

/// The operator that gives the same answer with its operands swapped.
fn mirrored(op: ComparisonOperator) -> ComparisonOperator {
    match op {
        ComparisonOperator::Less => ComparisonOperator::Greater,
        ComparisonOperator::LessEqual => ComparisonOperator::GreaterEqual,
        ComparisonOperator::Greater => ComparisonOperator::Less,
        ComparisonOperator::GreaterEqual => ComparisonOperator::LessEqual,
        other => other,
    }
}

/// The answer of a comparison between two literals, which is known before
/// the program runs.
pub fn fold_comparison(condition: &Expr) -> Option<bool> {
    let Expr::Compare { left, op, right } = condition else {
        return None;
    };
    let (Expr::Literal(left), Expr::Literal(right)) = (left.as_ref(), right.as_ref()) else {
        return None;
    };
    let ordering = match (left, right) {
        (Literal::Bool(left), Literal::Bool(right)) => {
            return match op {
                ComparisonOperator::Equal => Some(left == right),
                ComparisonOperator::NotEqual => Some(left != right),
                _ => None,
            };
        }
        (Literal::Int(left), Literal::Int(right)) => left.cmp(right),
        (Literal::Int(left), Literal::Float(right)) => (*left as f64).partial_cmp(right)?,
        (Literal::Float(left), Literal::Int(right)) => left.partial_cmp(&(*right as f64))?,
        (Literal::Float(left), Literal::Float(right)) => left.partial_cmp(right)?,
        _ => return None,
    };
    Some(match op {
        ComparisonOperator::Equal => ordering.is_eq(),
        ComparisonOperator::NotEqual => ordering.is_ne(),
        ComparisonOperator::Less => ordering.is_lt(),
        ComparisonOperator::LessEqual => ordering.is_le(),
        ComparisonOperator::Greater => ordering.is_gt(),
        ComparisonOperator::GreaterEqual => ordering.is_ge(),
    })
}

impl Lowerer<'_> {
    /// Emit the opening record of a branch or loop whose condition compares a
    /// native numeric variable with a number or another such variable.
    /// `label` names the construct (`if`/`while`) in error messages.
    pub fn begin_comparison(
        &mut self,
        tag: MessageTag,
        label: &str,
        left: &Expr,
        op: ComparisonOperator,
        right: &Expr,
    ) -> Result<(), String> {
        let (left, op, right) = if left.as_ident().is_none() && right.as_ident().is_some() {
            (right, mirrored(op), left)
        } else {
            (left, op, right)
        };
        let variable = left
            .as_ident()
            .and_then(|name| self.state.values.get(name).map(|value| (name, value)))
            .filter(|(_, value)| {
                value.native && (value.ty.is_numeric() || value.ty == Type::Boolean)
            });
        let Some((name, variable)) = variable else {
            return Err(format!(
                "{label} condition requires an integer or float value"
            ));
        };
        let operand = match right {
            Expr::Literal(Literal::Bool(flag)) if variable.ty == Type::Boolean => {
                Some(Operand::Number(if *flag { "1" } else { "0" }.to_owned()))
            }
            other => self.numeric_operand(other),
        };
        let condition = Comparison {
            left: name.to_owned(),
            operator: op,
            operand: operand
                .ok_or_else(|| format!("{label} condition requires a numeric literal or value"))?,
            type_name: variable.ty.name(),
        };
        emit::comparison(&mut self.state.output, tag, &condition);
        Ok(())
    }
}
