use ast::Expr;
use native::{Property, StateCheck};
use types::{ComparisonOperator, Literal};

use crate::context::Lowerer;
use crate::emit;

/// The check a condition makes on a property of a binding, if that is what it
/// is: `x.isOwned`, `!x.isOwned`, `x.isOwned == false` or `x.isOwned != true`.
pub fn state_check(condition: &Expr) -> Option<StateCheck> {
    match condition {
        Expr::Path(_) => property_of(condition, true),
        Expr::Not(inner) => property_of(inner, false),
        Expr::Compare { left, op, right } => {
            let Expr::Literal(Literal::Bool(value)) = right.as_ref() else {
                return None;
            };
            match op {
                ComparisonOperator::Equal => property_of(left, *value),
                ComparisonOperator::NotEqual => property_of(left, !*value),
                _ => None,
            }
        }
        _ => None,
    }
}

fn property_of(expression: &Expr, expected: bool) -> Option<StateCheck> {
    let Expr::Path(path) = expression else {
        return None;
    };
    let [slot, property] = path.segments() else {
        return None;
    };
    Some(StateCheck {
        slot: slot.clone(),
        property: property.parse::<Property>().ok()?,
        expected,
    })
}

impl Lowerer<'_> {
    /// Open an `if` on a property of a binding when `condition` is one.
    pub fn begin_state_check(&mut self, condition: &Expr) -> bool {
        let Some(check) = state_check(condition) else {
            return false;
        };
        emit::if_state(&mut self.state.output, &check);
        true
    }

    /// Lower `throw message;` to a record that fails the run in the VM.
    pub fn lower_throw(&mut self, message: &Expr) {
        let text = match message {
            Expr::Literal(Literal::String(text)) => text.clone(),
            other => other.to_string(),
        };
        emit::throw(&mut self.state.output, &text);
    }
}
