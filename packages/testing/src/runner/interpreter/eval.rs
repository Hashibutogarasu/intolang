use anyhow::{Result, anyhow, bail};
use ast::Expr;
use types::{Literal, Storage};

use super::execute::Interpreter;
use super::value::{Env, Value};
use crate::runner::builtins::IS_IN_MEMORY;

impl Interpreter<'_> {
    pub(super) fn eval(&self, expression: &Expr, env: &mut Env, line: usize) -> Result<Value> {
        match expression {
            Expr::Not(inner) => match self.eval(inner, env, line)? {
                Value::Bool(value) => Ok(Value::Bool(!value)),
                Value::Object(_) => bail!(
                    "{}:{line}: `!` expects Boolean in {}",
                    self.path,
                    self.test
                ),
            },
            Expr::Literal(Literal::Bool(value)) => Ok(Value::Bool(*value)),
            Expr::Call { callee, args } => {
                let arguments = args
                    .iter()
                    .map(|argument| self.eval(argument, env, line))
                    .collect::<Result<Vec<_>>>()?;
                self.call(&callee.to_string(), arguments, line)
            }
            Expr::Path(path) if path.last() == IS_IN_MEMORY && path.segments().len() == 2 => {
                match env.get_mut(path.first()) {
                    Some(Value::Object(object)) => {
                        let exists = match object.storage {
                            Storage::Now => object.allocated,
                            Storage::Late => {
                                object.allocated = true;
                                true
                            }
                            Storage::Local => object.in_init,
                        };
                        Ok(Value::Bool(exists))
                    }
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
