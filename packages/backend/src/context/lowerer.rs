use anyhow::Error;

use super::env::Env;
use super::state::State;
use crate::diagnostic::diagnostic;

/// Walks AST nodes against an [`Env`], appending the lowered message to its
/// [`State`]. Statement and expression handling is added to this type by the
/// `stmt` and `expr` modules.
pub struct Lowerer<'a> {
    pub env: &'a Env,
    pub state: State,
}

impl<'a> Lowerer<'a> {
    pub fn new(env: &'a Env) -> Self {
        Self {
            env,
            state: State::default(),
        }
    }

    /// A diagnostic bound to the entry source path and the current scope.
    pub fn diagnostic(&self, line: usize, message: &str) -> Error {
        diagnostic(
            &self.env.source_path,
            line,
            self.state.scope_name(),
            message,
        )
    }

    pub fn into_output(self) -> String {
        self.state.output
    }
}
