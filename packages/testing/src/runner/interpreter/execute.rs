use anyhow::{Result, bail};
use ast::{Expr, FunctionDecl, Stmt, StmtKind};
use std::collections::HashMap;
use types::{ComparisonOperator, Literal, Storage};

use super::value::{Env, Object, Value, set_init};
use crate::runner::builtins::ASSERT;

/// Runs a test body against the builtin Into definitions.
pub struct Interpreter<'a> {
    pub functions: &'a HashMap<String, FunctionDecl>,
    pub path: &'a str,
    pub test: &'a str,
}

impl Interpreter<'_> {
    pub fn run_test(&self, body: &[Stmt]) -> Result<()> {
        self.run_block(&mut Env::new(), body, false)
    }

    fn run_block(&self, env: &mut Env, body: &[Stmt], in_init: bool) -> Result<()> {
        for stmt in body {
            let line = stmt.line;
            match &stmt.kind {
                StmtKind::Declaration(declaration) if declaration.value.is_some() => {
                    let storage = declaration.storage;
                    env.insert(
                        declaration.name.clone(),
                        Value::Object(Object {
                            storage,
                            allocated: storage == Storage::Now,
                            in_init,
                        }),
                    );
                }
                StmtKind::Init(inner) => {
                    set_init(env, true);
                    self.run_block(env, inner, true)?;
                    set_init(env, false);
                }
                StmtKind::Unload(name) => {
                    if let Some(Value::Object(object)) = env.get_mut(name) {
                        object.allocated = false;
                    }
                }
                StmtKind::Expr(Expr::Call { callee, args })
                    if callee.as_ident() == Some(ASSERT) =>
                {
                    let value = match args.first() {
                        Some(argument) => self.eval(argument, env, line)?,
                        None => bail!(
                            "{}:{line}: undefined variable `` in {}",
                            self.path,
                            self.test
                        ),
                    };
                    self.call(ASSERT, vec![value], line)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn call(&self, name: &str, args: Vec<Value>, line: usize) -> Result<Value> {
        let Some(function) = self.functions.get(name) else {
            bail!(
                "{}:{line}: function `{name}` has no Into definition",
                self.path
            );
        };
        if function.params.len() != args.len() {
            bail!("{}:{line}: wrong argument count for {name}", self.path);
        }
        let mut env: Env = function
            .params
            .iter()
            .map(|param| param.name.clone())
            .zip(args)
            .collect();
        let body = function.body.as_deref().unwrap_or_default();
        Ok(self
            .run_function(body, &mut env, name, line)?
            .unwrap_or(Value::Bool(true)))
    }

    fn run_function(
        &self,
        body: &[Stmt],
        env: &mut Env,
        name: &str,
        line: usize,
    ) -> Result<Option<Value>> {
        for stmt in body {
            match &stmt.kind {
                StmtKind::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    let branch = if self.condition(condition, env, name, line)? {
                        Some(then_body.as_slice())
                    } else {
                        else_body.as_deref()
                    };
                    if let Some(branch) = branch
                        && let Some(value) = self.run_function(branch, env, name, line)?
                    {
                        return Ok(Some(value));
                    }
                }
                StmtKind::Throw(message) => {
                    let message = match message {
                        Expr::Literal(Literal::String(text)) => text.clone(),
                        other => other.to_string(),
                    };
                    bail!(
                        "{}:{line}: Into assertion failed in {}: {message}",
                        self.path,
                        self.test
                    );
                }
                StmtKind::Return(Some(expression)) => {
                    return Ok(Some(self.eval(expression, env, line)?));
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn condition(&self, condition: &Expr, env: &mut Env, name: &str, line: usize) -> Result<bool> {
        if let Expr::Compare { left, op, right } = condition {
            let (left, right) = (self.eval(left, env, line)?, self.eval(right, env, line)?);
            return match (left, right, op) {
                (Value::Bool(a), Value::Bool(b), ComparisonOperator::Equal) => Ok(a == b),
                (Value::Bool(a), Value::Bool(b), ComparisonOperator::NotEqual) => Ok(a != b),
                _ => bail!("{}:{line}: invalid Boolean condition in {name}", self.path),
            };
        }
        match self.eval(condition, env, line)? {
            Value::Bool(value) => Ok(value),
            Value::Object(_) => bail!(
                "{}:{line}: if condition must be Boolean in {name}",
                self.path
            ),
        }
    }
}
