use anyhow::{Result, anyhow, bail};
use ast::Expr;
use types::{Literal, Storage};

use super::execute::Interpreter;
use super::value::{Env, Object, State, Value};
use crate::runner::builtins::{
    IS_BORROWED, IS_DESTROYED, IS_IN_MEMORY, IS_MOVED, IS_OWNED, IS_REFERENCE, PROPERTIES,
};

impl Interpreter<'_> {
    pub(super) fn eval(&self, expression: &Expr, env: &mut Env, line: usize) -> Result<Value> {
        match expression {
            Expr::Not(inner) => match self.eval(inner, env, line)? {
                Value::Bool(value) => Ok(Value::Bool(!value)),
                _ => {
                    bail!("{}:{line}: `!` expects Boolean in {}", self.path, self.test)
                }
            },
            Expr::Literal(Literal::Bool(value)) => Ok(Value::Bool(*value)),
            Expr::Literal(Literal::Int(value)) => Ok(Value::Int(*value)),
            Expr::Call { callee, args } => {
                let arguments = args
                    .iter()
                    .map(|argument| self.eval(argument, env, line))
                    .collect::<Result<Vec<_>>>()?;
                self.call(&callee.to_string(), arguments, line)
            }
            Expr::Path(path) if path.segments().len() == 2 && PROPERTIES.contains(&path.last()) => {
                match env.get_mut(path.first()) {
                    Some(Value::Object(object)) => Ok(Value::Bool(property(object, path.last()))),
                    _ => Err(self.undefined(&path.to_string(), line)),
                }
            }
            other => env
                .get(&other.to_string())
                .cloned()
                .ok_or_else(|| self.undefined(&other.to_string(), line)),
        }
    }

    fn undefined(&self, name: &str, line: usize) -> anyhow::Error {
        anyhow!(
            "{}:{line}: undefined variable `{name}` in {}",
            self.path,
            self.test
        )
    }
}

fn property(object: &mut Object, property: &str) -> bool {
    match property {
        IS_IN_MEMORY => {
            if object.state != State::Live {
                return false;
            }
            match object.storage {
                Storage::Now => object.allocated,
                Storage::Late => {
                    object.allocated = true;
                    true
                }
                Storage::Local => object.in_init,
            }
        }
        IS_OWNED => object.state == State::Live && !object.reference,
        IS_REFERENCE => object.reference,
        IS_BORROWED => object.borrowed_by > 0,
        IS_MOVED => object.state == State::Moved,
        IS_DESTROYED => object.state == State::Destroyed,
        _ => false,
    }
}
