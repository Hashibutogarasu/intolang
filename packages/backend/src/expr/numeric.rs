use ast::Expr;
use native::{CompareAssign, Comparison, Operand};
use parser::ArithmeticLexeme;
use types::{ArithmeticOperator, ComparisonOperator, Literal, LogicalOperator, MessageTag, Type};

use crate::context::{Lowerer, Variable};
use crate::emit;

/// What decides the type of a lowered operand.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Typed(Type),
    IntLiteral,
    FloatLiteral,
}

/// A lowered operand: a number or the slot that holds it.
#[derive(Debug, Clone)]
pub struct Numeric {
    pub operand: Operand,
    pub kind: Kind,
}

fn integer_only(operator: ArithmeticOperator) -> bool {
    matches!(
        operator,
        ArithmeticOperator::Rem
            | ArithmeticOperator::BitAnd
            | ArithmeticOperator::BitOr
            | ArithmeticOperator::BitXor
            | ArithmeticOperator::ShiftLeft
            | ArithmeticOperator::ShiftRight
    )
}

impl Lowerer<'_> {
    fn fresh_slot(&mut self, ty: &Type) -> String {
        let name = format!("__value{}", self.state.temp_counter);
        self.state.temp_counter += 1;
        self.state
            .values
            .insert(name.clone(), Variable::temporary(ty.clone()));
        name
    }

    /// The type two operands share. A literal takes the type of the other
    /// operand.
    fn unify(&self, left: &Kind, right: &Kind) -> Result<Type, String> {
        let mismatch = |a: &Type, b: &Type| {
            format!(
                "Arithmetic type `{}` does not match operand type `{}`",
                a.name(),
                b.name()
            )
        };
        match (left, right) {
            (Kind::Typed(a), Kind::Typed(b)) if a == b => Ok(a.clone()),
            (Kind::Typed(a), Kind::Typed(b)) => Err(mismatch(a, b)),
            (Kind::Typed(ty), Kind::IntLiteral) | (Kind::IntLiteral, Kind::Typed(ty)) => {
                if ty.is_numeric() || *ty == Type::Boolean {
                    Ok(ty.clone())
                } else {
                    Err(mismatch(ty, &Type::Int32))
                }
            }
            (Kind::Typed(ty), Kind::FloatLiteral) | (Kind::FloatLiteral, Kind::Typed(ty)) => {
                if ty.is_float() {
                    Ok(ty.clone())
                } else {
                    Err(mismatch(ty, &Type::Float64))
                }
            }
            (Kind::IntLiteral, Kind::IntLiteral) => Ok(Type::Int32),
            _ => Ok(Type::Float64),
        }
    }

    /// The slot holding `numeric`; a number is stored into a new slot of `ty`.
    fn slot_of(&mut self, numeric: &Numeric, ty: &Type) -> String {
        match &numeric.operand {
            Operand::Slot(name) => name.clone(),
            Operand::Number(value) => {
                let slot = self.fresh_slot(ty);
                emit::assign(&mut self.state.output, &slot, ty, "n", value);
                slot
            }
            Operand::Str(_) => unreachable!("numeric operands are never text"),
        }
    }

    /// Lower an arithmetic expression, storing its value in `target`. The
    /// annotation, when given, has to be the type of the operands.
    pub fn lower_binary_into(
        &mut self,
        left: &Expr,
        operator: ArithmeticOperator,
        right: &Expr,
        annotation: Option<&Type>,
        target: &str,
    ) -> Result<Type, String> {
        let left = self.lower_numeric(left)?;
        let right = self.lower_numeric(right)?;
        let literals = matches!(
            (&left.kind, &right.kind),
            (
                Kind::IntLiteral | Kind::FloatLiteral,
                Kind::IntLiteral | Kind::FloatLiteral
            )
        );
        let ty = match annotation {
            Some(annotation) if literals && annotation.is_numeric() => annotation.clone(),
            Some(annotation) => {
                let ty = self.unify(&left.kind, &right.kind)?;
                if *annotation != ty {
                    return Err(format!(
                        "Arithmetic type `{}` does not match operand type `{}`",
                        annotation.name(),
                        ty.name()
                    ));
                }
                ty
            }
            None => self.unify(&left.kind, &right.kind)?,
        };
        if !ty.is_numeric() {
            return Err("Arithmetic result type must be numeric".to_owned());
        }
        if integer_only(operator) && ty.is_float() {
            return Err(format!(
                "`{}` needs integers but received `{}`",
                ArithmeticLexeme::from(operator),
                ty.name()
            ));
        }
        let left_slot = self.slot_of(&left, &ty);
        emit::arith(
            &mut self.state.output,
            target,
            &left_slot,
            ArithmeticLexeme::from(operator).message_name(),
            &right.operand,
            &ty,
        );
        Ok(ty)
    }

    /// Lower a numeric or Boolean expression to an operand.
    pub fn lower_numeric(&mut self, expression: &Expr) -> Result<Numeric, String> {
        match expression {
            Expr::Literal(Literal::Int(number)) => Ok(Numeric {
                operand: Operand::Number(number.to_string()),
                kind: Kind::IntLiteral,
            }),
            Expr::Literal(Literal::Float(number)) => Ok(Numeric {
                operand: Operand::Number(number.to_string()),
                kind: Kind::FloatLiteral,
            }),
            Expr::Literal(Literal::Bool(flag)) => Ok(Numeric {
                operand: Operand::Number(if *flag { "1" } else { "0" }.to_owned()),
                kind: Kind::Typed(Type::Boolean),
            }),
            Expr::Binary { left, op, right } => {
                let target = format!("__value{}", self.state.temp_counter);
                self.state.temp_counter += 1;
                let ty = self.lower_binary_into(left, *op, right, None, &target)?;
                self.state
                    .values
                    .insert(target.clone(), Variable::temporary(ty.clone()));
                Ok(Numeric {
                    operand: Operand::Slot(target),
                    kind: Kind::Typed(ty),
                })
            }
            Expr::Compare { .. } | Expr::Logical { .. } | Expr::Not(_) => {
                let slot = self.lower_boolean(expression)?;
                Ok(Numeric {
                    operand: Operand::Slot(slot),
                    kind: Kind::Typed(Type::Boolean),
                })
            }
            other => self.lower_value(other),
        }
    }

    /// Lower a name or a call whose value lives in a native slot.
    fn lower_value(&mut self, expression: &Expr) -> Result<Numeric, String> {
        if let Some(name) = expression.as_ident() {
            self.ensure_usable(name)?;
            let variable = self
                .state
                .values
                .get(name)
                .filter(|variable| variable.native)
                .ok_or_else(|| "Arithmetic requires a typed numeric value".to_owned())?;
            if !(variable.ty.is_numeric() || variable.ty == Type::Boolean) {
                return Err("Arithmetic requires a typed numeric value".to_owned());
            }
            return Ok(Numeric {
                operand: Operand::Slot(name.to_owned()),
                kind: Kind::Typed(variable.ty.clone()),
            });
        }
        if !matches!(expression, Expr::Call { .. }) {
            return Err(format!("Unsupported arithmetic operand `{expression}`"));
        }
        let target = format!("__value{}", self.state.temp_counter);
        self.state.temp_counter += 1;
        let env = self.env;
        let (_, ty, native) =
            self.declaration_value(expression, None, &env.type_params, &target)?;
        if !native || !(ty.is_numeric() || ty == Type::Boolean) {
            return Err(format!("Unsupported arithmetic operand `{expression}`"));
        }
        self.state
            .values
            .insert(target.clone(), Variable::temporary(ty.clone()));
        Ok(Numeric {
            operand: Operand::Slot(target),
            kind: Kind::Typed(ty),
        })
    }

    /// Lower a Boolean expression to a `Boolean` slot holding 1 or 0.
    pub fn lower_boolean(&mut self, expression: &Expr) -> Result<String, String> {
        match expression {
            Expr::Not(inner) => {
                let value = self.lower_boolean(inner)?;
                let target = self.fresh_slot(&Type::Boolean);
                self.emit_compare(&target, &value, ComparisonOperator::Equal, "0");
                Ok(target)
            }
            Expr::Compare { left, op, right } => {
                let left = self.lower_numeric(left)?;
                let right = self.lower_numeric(right)?;
                let ty = self.unify(&left.kind, &right.kind)?;
                let left_slot = self.slot_of(&left, &ty);
                let target = self.fresh_slot(&Type::Boolean);
                let comparison = Comparison {
                    left: left_slot,
                    operator: *op,
                    operand: right.operand,
                    type_name: ty.name(),
                };
                emit::compare_assign(
                    &mut self.state.output,
                    &CompareAssign {
                        target: target.clone(),
                        comparison,
                    },
                );
                Ok(target)
            }
            Expr::Logical { left, op, right } => {
                let first = self.lower_boolean(left)?;
                let result = self.fresh_slot(&Type::Boolean);
                let (start, enter) = match op {
                    LogicalOperator::And => ("0", "1"),
                    LogicalOperator::Or => ("1", "0"),
                };
                emit::assign(&mut self.state.output, &result, &Type::Boolean, "n", start);
                let guard = Comparison {
                    left: first,
                    operator: ComparisonOperator::Equal,
                    operand: Operand::Number(enter.to_owned()),
                    type_name: Type::Boolean.name(),
                };
                emit::comparison(&mut self.state.output, MessageTag::IfBegin, &guard);
                let second = self.lower_boolean(right)?;
                emit::assign(
                    &mut self.state.output,
                    &result,
                    &Type::Boolean,
                    "p",
                    &second,
                );
                emit::bare(&mut self.state.output, MessageTag::IfEnd);
                Ok(result)
            }
            other => {
                let value = self.lower_numeric(other)?;
                match (&value.kind, &value.operand) {
                    (Kind::Typed(Type::Boolean), Operand::Slot(name)) => Ok(name.clone()),
                    (Kind::Typed(Type::Boolean), Operand::Number(number)) => {
                        let slot = self.fresh_slot(&Type::Boolean);
                        emit::assign(&mut self.state.output, &slot, &Type::Boolean, "n", number);
                        Ok(slot)
                    }
                    _ => Err(format!("`{other}` is not a Boolean expression")),
                }
            }
        }
    }

    fn emit_compare(
        &mut self,
        target: &str,
        left: &str,
        operator: ComparisonOperator,
        number: &str,
    ) {
        let comparison = Comparison {
            left: left.to_owned(),
            operator,
            operand: Operand::Number(number.to_owned()),
            type_name: Type::Boolean.name(),
        };
        emit::compare_assign(
            &mut self.state.output,
            &CompareAssign {
                target: target.to_owned(),
                comparison,
            },
        );
    }

    /// Open a branch or loop on a Boolean expression that is not a plain
    /// comparison of a variable.
    pub fn begin_boolean_branch(
        &mut self,
        tag: MessageTag,
        expression: &Expr,
    ) -> Result<String, String> {
        let slot = self.lower_boolean(expression)?;
        let condition = Comparison {
            left: slot.clone(),
            operator: ComparisonOperator::Equal,
            operand: Operand::Number("1".to_owned()),
            type_name: Type::Boolean.name(),
        };
        emit::comparison(&mut self.state.output, tag, &condition);
        Ok(slot)
    }
}
