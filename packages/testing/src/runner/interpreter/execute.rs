use anyhow::{Result, bail};
use ast::{Declaration, Expr, FunctionDecl, Stmt, StmtKind};
use std::collections::HashMap;
use types::{ComparisonOperator, Literal, Storage};

use super::value::{Env, Object, State, Value, set_init};
use crate::runner::builtins::{BORROW, COPY, DESTROY, MOVE};

/// Runs a test body against the builtin Into definitions.
pub struct Interpreter<'a> {
    pub functions: &'a HashMap<String, FunctionDecl>,
    pub path: &'a str,
    pub test: &'a str,
}

/// The functions whose calls a test body must be able to resolve.
const ASSERTION_PREFIX: &str = "assert";

impl Interpreter<'_> {
    pub fn run_test(&self, body: &[Stmt]) -> Result<()> {
        self.run_block(&mut Env::new(), body, false)
    }

    fn run_block(&self, env: &mut Env, body: &[Stmt], in_init: bool) -> Result<()> {
        for stmt in body {
            let line = stmt.line;
            match &stmt.kind {
                StmtKind::Declaration(declaration) if declaration.value.is_some() => {
                    self.declare(env, declaration, in_init, line)?;
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
                StmtKind::Expr(Expr::Call { callee, args }) => {
                    if let [receiver, method] = callee.segments()
                        && matches!(env.get(receiver), Some(Value::Object(_)))
                    {
                        self.object_call(env, receiver, method, line)?;
                    } else if let Some(name) = callee.as_ident()
                        && name.starts_with(ASSERTION_PREFIX)
                    {
                        let values = args
                            .iter()
                            .map(|argument| self.eval(argument, env, line))
                            .collect::<Result<Vec<_>>>()?;
                        self.call(name, values, line)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn declare(
        &self,
        env: &mut Env,
        declaration: &Declaration,
        in_init: bool,
        line: usize,
    ) -> Result<()> {
        let storage = declaration.storage;
        if let Some(Expr::Call { callee, args }) = &declaration.value
            && let [receiver, method] = callee.segments()
            && args.is_empty()
            && matches!(method.as_str(), COPY | MOVE | BORROW)
            && matches!(env.get(receiver), Some(Value::Object(_)))
        {
            let object = self.take_ownership(env, receiver, method, storage, in_init, line)?;
            env.insert(declaration.name.clone(), Value::Object(object));
            return Ok(());
        }
        let number = match &declaration.value {
            Some(Expr::Literal(Literal::Int(number))) => Some(*number),
            _ => None,
        };
        env.insert(
            declaration.name.clone(),
            Value::Object(Object::declared(storage, in_init, number)),
        );
        Ok(())
    }

    fn live<'e>(&self, env: &'e mut Env, name: &str, line: usize) -> Result<&'e mut Object> {
        let Some(Value::Object(object)) = env.get_mut(name) else {
            bail!(
                "{}:{line}: undefined variable `{name}` in {}",
                self.path,
                self.test
            );
        };
        match object.state {
            State::Live => Ok(object),
            State::Moved => bail!("{}:{line}: `{name}` was moved in {}", self.path, self.test),
            State::Destroyed => {
                bail!(
                    "{}:{line}: `{name}` was destroyed in {}",
                    self.path,
                    self.test
                )
            }
        }
    }

    /// The binding `x.copy()`, `x.move()` or `x.borrow()` produces.
    fn take_ownership(
        &self,
        env: &mut Env,
        name: &str,
        method: &str,
        storage: Storage,
        in_init: bool,
        line: usize,
    ) -> Result<Object> {
        let source = self.live(env, name, line)?;
        let mut object = Object::declared(storage, in_init, source.number);
        object.allocated = true;
        match method {
            COPY => {}
            MOVE => {
                if source.borrowed_by > 0 {
                    bail!(
                        "{}:{line}: `{name}` is borrowed in {}",
                        self.path,
                        self.test
                    );
                }
                source.state = State::Moved;
                source.allocated = false;
            }
            _ => {
                source.borrowed_by += 1;
                object.reference = true;
                object.borrows = Some(name.to_owned());
            }
        }
        Ok(object)
    }

    /// A method call on a binding that changes what it owns.
    fn object_call(&self, env: &mut Env, name: &str, method: &str, line: usize) -> Result<()> {
        match method {
            DESTROY => {
                let object = self.live(env, name, line)?;
                if object.reference {
                    let owner = object.borrows.clone();
                    object.state = State::Destroyed;
                    object.allocated = false;
                    if let Some(Value::Object(owner)) = owner.and_then(|owner| env.get_mut(&owner))
                    {
                        owner.borrowed_by = owner.borrowed_by.saturating_sub(1);
                    }
                } else if object.borrowed_by > 0 {
                    bail!(
                        "{}:{line}: `{name}` is borrowed in {}",
                        self.path,
                        self.test
                    );
                } else {
                    object.state = State::Destroyed;
                    object.allocated = false;
                }
            }
            MOVE => {
                let object = self.live(env, name, line)?;
                if object.borrowed_by > 0 {
                    bail!(
                        "{}:{line}: `{name}` is borrowed in {}",
                        self.path,
                        self.test
                    );
                }
                object.state = State::Moved;
                object.allocated = false;
            }
            _ => {}
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
            if let (Value::Bool(a), Value::Bool(b)) = (&left, &right) {
                return match op {
                    ComparisonOperator::Equal => Ok(a == b),
                    ComparisonOperator::NotEqual => Ok(a != b),
                    _ => bail!("{}:{line}: invalid Boolean condition in {name}", self.path),
                };
            }
            if let (Some(a), Some(b)) = (left.number(), right.number()) {
                return Ok(match op {
                    ComparisonOperator::Equal => a == b,
                    ComparisonOperator::NotEqual => a != b,
                    ComparisonOperator::Less => a < b,
                    ComparisonOperator::LessEqual => a <= b,
                    ComparisonOperator::Greater => a > b,
                    ComparisonOperator::GreaterEqual => a >= b,
                });
            }
            bail!("{}:{line}: invalid condition in {name}", self.path);
        }
        match self.eval(condition, env, line)? {
            Value::Bool(value) => Ok(value),
            _ => bail!(
                "{}:{line}: if condition must be Boolean in {name}",
                self.path
            ),
        }
    }
}
