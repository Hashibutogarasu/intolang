use errors::{Errors, Result};
use types::ComparisonOperator;

use crate::value::Value;

pub fn is_float(ty: &str) -> bool {
    matches!(ty, "Float32" | "Float64")
}

/// Wrap an integer to the width of the type that holds it.
pub fn wrap(ty: &str, number: i64) -> i64 {
    match ty {
        "Int8" => number as i8 as i64,
        "Int16" => number as i16 as i64,
        "Int32" => number as i32 as i64,
        "Boolean" => number & 1,
        _ => number,
    }
}

pub fn constant(ty: &str, text: &str) -> Result<Value> {
    if is_float(ty) {
        return Ok(Value::Float(text.parse()?));
    }
    Ok(Value::Int(wrap(ty, text.parse()?)))
}

pub fn operate(ty: &str, operation: &str, left: &Value, right: &Value) -> Result<Value> {
    if is_float(ty) {
        let (Some(a), Some(b)) = (left.float(), right.float()) else {
            return Err(errors::Error::from(Errors::ArithmeticNeedsNumbersVm));
        };
        let result = match operation {
            "add" => a + b,
            "sub" => a - b,
            "mul" => a * b,
            "div" => a / b,
            "rem" => a % b,
            other => {
                return Err(errors::Error::from(
                    Errors::UnsupportedArithmeticOperation {
                        operation: other.to_string(),
                    },
                ));
            }
        };
        return Ok(Value::Float(if ty == "Float32" {
            result as f32 as f64
        } else {
            result
        }));
    }
    let (Some(a), Some(b)) = (left.int(), right.int()) else {
        return Err(errors::Error::from(Errors::ArithmeticNeedsNumbersVm));
    };
    let result = match operation {
        "add" => a.wrapping_add(b),
        "sub" => a.wrapping_sub(b),
        "mul" => a.wrapping_mul(b),
        "div" => {
            if b == 0 {
                return Err(errors::Error::from(Errors::DivisionByZero));
            }
            a.wrapping_div(b)
        }
        "rem" => {
            if b == 0 {
                return Err(errors::Error::from(Errors::DivisionByZero));
            }
            a.wrapping_rem(b)
        }
        "and" => a & b,
        "or" => a | b,
        "xor" => a ^ b,
        "shl" => a.wrapping_shl(u32::try_from(b).unwrap_or(u32::MAX)),
        "shr" => a.wrapping_shr(u32::try_from(b).unwrap_or(u32::MAX)),
        other => {
            return Err(errors::Error::from(
                Errors::UnsupportedArithmeticOperation {
                    operation: other.to_string(),
                },
            ));
        }
    };
    Ok(Value::Int(wrap(ty, result)))
}

/// Convert a scalar from the type `from` to the type `to`.
pub fn convert(from: &str, to: &str, value: &Value) -> Result<Value> {
    if is_float(to) {
        let Some(number) = value.float() else {
            return Err(errors::Error::from(Errors::CannotCast {
                from: from.to_string(),
                to: to.to_string(),
            }));
        };
        return Ok(Value::Float(if to == "Float32" {
            number as f32 as f64
        } else {
            number
        }));
    }
    let number = match value {
        Value::Int(number) => *number,
        Value::Float(number) => *number as i64,
        _ => {
            return Err(errors::Error::from(Errors::CannotCast {
                from: from.to_string(),
                to: to.to_string(),
            }));
        }
    };
    Ok(Value::Int(wrap(to, number)))
}

pub fn compare(operator: ComparisonOperator, left: &Value, right: &Value) -> Result<bool> {
    let ordering = match (left, right) {
        (Value::Int(a), Value::Int(b)) => a.cmp(b),
        (Value::Str(a), Value::Str(b)) => a.cmp(b),
        (Value::Ref(a), Value::Ref(b)) => a.cmp(b),
        _ => match (left.float(), right.float()) {
            (Some(a), Some(b)) => match a.partial_cmp(&b) {
                Some(ordering) => ordering,
                None => return Ok(operator == ComparisonOperator::NotEqual),
            },
            _ => return Err(errors::Error::from(Errors::KindsNotComparable)),
        },
    };
    Ok(match operator {
        ComparisonOperator::Equal => ordering.is_eq(),
        ComparisonOperator::NotEqual => ordering.is_ne(),
        ComparisonOperator::Less => ordering.is_lt(),
        ComparisonOperator::LessEqual => ordering.is_le(),
        ComparisonOperator::Greater => ordering.is_gt(),
        ComparisonOperator::GreaterEqual => ordering.is_ge(),
    })
}
