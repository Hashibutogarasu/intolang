use ast::{Expr, Stmt};
use native::NativeLink;
use parser::Hex;
use types::{ArithmeticOperator, ComparisonOperator, Literal, MessageTag, Type};

use super::captures::assigned_names;
use crate::context::Lowerer;
use crate::emit;

const CONCAT: &str = "String.concat";
const FROM_CHAR: &str = "String.fromChar";
const EQUALS: &str = "String.equals";

/// A lowered `String` value: text known while lowering, or the slot that
/// holds the text at run time.
pub enum Text {
    Const(String),
    Slot(String),
}

impl Text {
    fn argument(&self) -> Expr {
        match self {
            Text::Const(value) => Expr::Literal(Literal::String(value.clone())),
            Text::Slot(name) => Expr::ident(name.clone()),
        }
    }
}

impl<'a> Lowerer<'a> {
    /// Give the texts that `body` assigns to a slot of their own before the
    /// branch or loop starts, because a branch or an iteration may change
    /// them and later reads must see the value of the run.
    pub fn materialize_assigned_texts(&mut self, body: &[Stmt]) {
        for name in assigned_names(body) {
            let Some(variable) = self.state.values.get(&name) else {
                continue;
            };
            if variable.native || variable.ty != Type::String {
                continue;
            }
            let hex = Hex::from(variable.value.as_str()).to_string();
            emit::assign(&mut self.state.output, &name, &Type::String, "s", &hex);
            if let Some(variable) = self.state.values.get_mut(&name) {
                variable.native = true;
                variable.value = Type::String.name();
            }
        }
    }

    /// Whether the expression is a `String`.
    pub fn is_text(&self, expression: &Expr) -> bool {
        matches!(self.type_of(expression), Ok(Type::String))
    }

    fn string_link(&self, key: &str) -> Result<&'a NativeLink, String> {
        let env = self.env;
        env.native_links
            .get(key)
            .ok_or_else(|| format!("`{key}` is not available; import \"intolang://builtin/std\""))
    }

    /// Lower a `String` expression. A concatenation of known text stays known
    /// text; otherwise the result goes to `target`, or to a new slot.
    pub fn lower_text(&mut self, expression: &Expr, target: Option<&str>) -> Result<Text, String> {
        match expression {
            Expr::Literal(Literal::String(value)) => Ok(Text::Const(value.clone())),
            Expr::Literal(Literal::Char(value)) => Ok(Text::Const(value.to_string())),
            Expr::Binary {
                left,
                op: ArithmeticOperator::Add,
                right,
            } => {
                let left = self.lower_text(left, None)?;
                let right = self.lower_text(right, None)?;
                self.concat(left, right, target)
            }
            other => self.text_operand(other),
        }
    }

    fn concat(&mut self, left: Text, right: Text, target: Option<&str>) -> Result<Text, String> {
        if let (Text::Const(left), Text::Const(right)) = (&left, &right) {
            return Ok(Text::Const(format!("{left}{right}")));
        }
        let link = self.string_link(CONCAT)?;
        let destination = match target {
            Some(name) => name.to_owned(),
            None => self.fresh_slot(&Type::String),
        };
        self.emit_link_call(
            MessageTag::LinkAssign,
            link,
            vec![left.argument(), right.argument()],
            &destination,
            false,
        )?;
        Ok(Text::Slot(destination))
    }

    fn text_operand(&mut self, expression: &Expr) -> Result<Text, String> {
        if let Some(name) = expression.as_ident() {
            self.ensure_usable(name)?;
            let variable = self
                .state
                .values
                .get(name)
                .ok_or_else(|| format!("Unknown name `{name}`"))?;
            return match (variable.native, &variable.ty) {
                (false, _) => Ok(Text::Const(variable.value.clone())),
                (true, Type::String | Type::Path) => Ok(Text::Slot(name.to_owned())),
                (true, Type::Char) => self.char_text(name),
                (true, other) => Err(format!("`{name}` is a `{}`, not text", other.name())),
            };
        }
        if let Expr::Path(path) = expression
            && let Some((slot, ty)) = self.lower_field_read(path)?
        {
            return match ty {
                Type::String | Type::Path => Ok(Text::Slot(slot)),
                other => Err(format!("`{expression}` is a `{}`, not text", other.name())),
            };
        }
        if let Expr::Path(path) = expression
            && let Some(Literal::String(value)) = self
                .static_field(path)
                .and_then(|field| field.default_literal())
        {
            return Ok(Text::Const(value.clone()));
        }
        if matches!(self.type_of(expression), Ok(Type::Char)) {
            let numeric = self.lower_numeric(expression)?;
            if let native::Operand::Slot(slot) = numeric.operand {
                return self.char_text(&slot);
            }
        }
        if matches!(expression, Expr::Call { .. }) {
            let env = self.env;
            let temporary = self.fresh_slot(&Type::String);
            let (value, ty, native) =
                self.declaration_value(expression, None, &env.type_params, &temporary)?;
            return match (native, ty) {
                (true, Type::String | Type::Path) => Ok(Text::Slot(temporary)),
                (false, Type::String | Type::Path) => Ok(Text::Const(value)),
                (_, other) => Err(format!("`{expression}` is a `{}`, not text", other.name())),
            };
        }
        Err(format!("Unsupported text operand `{expression}`"))
    }

    fn char_text(&mut self, slot: &str) -> Result<Text, String> {
        let link = self.string_link(FROM_CHAR)?;
        let destination = self.fresh_slot(&Type::String);
        self.emit_link_call(
            MessageTag::LinkAssign,
            link,
            vec![Expr::ident(slot)],
            &destination,
            false,
        )?;
        Ok(Text::Slot(destination))
    }

    /// Lower `left == right` or `left != right` between texts to a `Boolean`
    /// slot.
    pub fn lower_text_comparison(
        &mut self,
        left: &Expr,
        op: ComparisonOperator,
        right: &Expr,
    ) -> Result<String, String> {
        if !matches!(op, ComparisonOperator::Equal | ComparisonOperator::NotEqual) {
            return Err("Texts can only be compared with `==` and `!=`".to_owned());
        }
        for side in [left, right] {
            if !(self.is_text(side) || matches!(self.type_of(side), Ok(Type::Char))) {
                return Err(format!(
                    "A text can only be compared with a text, not `{side}`"
                ));
            }
        }
        let left = self.lower_text(left, None)?;
        let right = self.lower_text(right, None)?;
        let equal = self.fresh_slot(&Type::Boolean);
        if let (Text::Const(left), Text::Const(right)) = (&left, &right) {
            let value = if left == right { "1" } else { "0" };
            emit::assign(&mut self.state.output, &equal, &Type::Boolean, "n", value);
        } else {
            let link = self.string_link(EQUALS)?;
            self.emit_link_call(
                MessageTag::LinkAssign,
                link,
                vec![left.argument(), right.argument()],
                &equal,
                false,
            )?;
        }
        if op == ComparisonOperator::Equal {
            return Ok(equal);
        }
        let different = self.fresh_slot(&Type::Boolean);
        self.emit_compare(&different, &equal, ComparisonOperator::Equal, "0");
        Ok(different)
    }
}
