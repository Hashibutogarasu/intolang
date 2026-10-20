use ast::{Expr, Path, Stmt};
use errors::{Errors, Result};
use types::Type;

use super::events::RuntimeCall;
use crate::context::{Lowerer, TASK, THREAD};
use crate::emit;

const RUN: &str = "run";

/// The value of a `thread { .. }` expression: the `Thread` that runs the
/// handler function `function`.
pub fn thread_value(function: &str) -> (String, Type, bool) {
    (function.to_owned(), Type::Named(THREAD.to_owned()), false)
}

/// The value `thread.run()` binds: a `Task` held in a native slot.
pub fn task_value() -> (String, Type, bool) {
    (TASK.to_owned(), Type::Named(TASK.to_owned()), true)
}

impl Lowerer<'_> {
    /// Lower the body of a `thread { .. }` into a function and return its
    /// name.
    pub fn lower_thread_body(&mut self, body: &[Stmt]) -> Result<String> {
        let name = format!("__thread_{}", self.state.temp_counter);
        self.state.temp_counter += 1;
        self.lower_handler(&name, &[], &[], body)?;
        Ok(name)
    }

    /// Lower `thread.run()`: start the thread and keep its task in the slot
    /// `assigned_to`.
    pub fn lower_thread_call(&mut self, callee: &Path, assigned_to: &str) -> Result<RuntimeCall> {
        let [receiver, method] = callee.segments() else {
            return Ok(RuntimeCall::NotRuntime);
        };
        let Some(variable) = self.state.values.get(receiver) else {
            return Ok(RuntimeCall::NotRuntime);
        };
        if variable.ty.name() != THREAD || method != RUN {
            return Ok(RuntimeCall::NotRuntime);
        }
        let function = variable.value.clone();
        self.ensure_usable(receiver)?;
        emit::thread_run(&mut self.state.output, assigned_to, &function);
        Ok(RuntimeCall::Task)
    }

    /// Lower `await task` and `await thread.run()`.
    pub fn lower_await(&mut self, awaited: &Expr) -> Result<()> {
        match awaited {
            Expr::Path(path) => {
                let task = path
                    .as_ident()
                    .filter(|name| {
                        self.state
                            .values
                            .get(*name)
                            .is_some_and(|variable| variable.ty.name() == TASK)
                    })
                    .ok_or_else(|| {
                        errors::Error::from(Errors::AwaitExpectsTask {
                            value: awaited.to_string(),
                        })
                    })?;
                self.ensure_usable(task)?;
                emit::await_task(&mut self.state.output, task);
                Ok(())
            }
            Expr::Call { callee, .. } => {
                let task = format!("__task{}", self.state.temp_counter);
                self.state.temp_counter += 1;
                match self.lower_thread_call(callee, &task)? {
                    RuntimeCall::Task => {
                        emit::await_task(&mut self.state.output, &task);
                        Ok(())
                    }
                    _ => Err(Errors::AwaitExpectsTask {
                        value: awaited.to_string(),
                    }
                    .into()),
                }
            }
            other => Err(Errors::AwaitExpectsTask {
                value: other.to_string(),
            }
            .into()),
        }
    }
}
