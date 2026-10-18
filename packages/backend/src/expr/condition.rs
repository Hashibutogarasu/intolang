use ast::Expr;
use native::Comparison;
use types::{ComparisonOperator, MessageTag};

use crate::context::Lowerer;
use crate::emit;

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
        let variable = left
            .as_ident()
            .and_then(|name| self.state.values.get(name).map(|value| (name, value)))
            .filter(|(_, value)| value.native && value.ty.is_numeric());
        let Some((name, variable)) = variable else {
            return Err(format!(
                "{label} condition requires an integer or float value"
            ));
        };
        let condition = Comparison {
            left: name.to_owned(),
            operator: op,
            operand: self
                .numeric_operand(right)
                .ok_or_else(|| format!("{label} condition requires a numeric literal or value"))?,
            type_name: variable.ty.name(),
        };
        emit::comparison(&mut self.state.output, tag, &condition);
        Ok(())
    }
}
