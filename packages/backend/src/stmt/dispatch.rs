use ast::{Stmt, StmtKind};
use errors::{Error, Errors, Result};
use types::MessageTag;

use crate::context::Lowerer;
use crate::emit;

impl Lowerer<'_> {
    pub fn lower_block(&mut self, body: &[Stmt]) -> Result<()> {
        body.iter().try_for_each(|stmt| self.lower_stmt(stmt))
    }

    /// Lower the statements of a scope. When errors are collected, a failing
    /// statement is recorded and the next one is lowered.
    pub fn lower_statements(&mut self, body: &[Stmt]) -> Result<()> {
        body.iter().try_for_each(|stmt| self.lower_collecting(stmt))
    }

    pub fn lower_collecting(&mut self, stmt: &Stmt) -> Result<()> {
        match self.lower_stmt(stmt) {
            Err(error) if self.state.collected.is_some() && self.state.frames.is_empty() => {
                self.collect(stmt, error);
                Ok(())
            }
            lowered => lowered,
        }
    }

    fn collect(&mut self, stmt: &Stmt, error: Error) {
        if let StmtKind::Declaration(declaration) = &stmt.kind {
            self.state.poisoned.insert(declaration.name.clone());
        }
        let follows_failure = match error.kind() {
            Errors::UndefinedName { name }
            | Errors::UndefinedVariable { name }
            | Errors::VariableOutOfScope { name } => self.state.poisoned.contains(name),
            _ => false,
        };
        if !follows_failure && let Some(collected) = self.state.collected.as_mut() {
            collected.push(error);
        }
    }

    /// Lower one statement. Statements the lowering has no meaning for, such
    /// as a call to a function that was never imported, produce no output.
    pub fn lower_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        let line = stmt.line;
        match &stmt.kind {
            StmtKind::Declaration(declaration) => self.lower_declaration(line, declaration),
            StmtKind::Assign { target, value } => self.lower_assignment(line, target, value),
            StmtKind::Unload(name) => {
                self.lower_unload(name);
                Ok(())
            }
            StmtKind::If {
                condition,
                then_body,
                else_body,
            } => self.lower_if(line, condition, then_body, else_body.as_deref()),
            StmtKind::While { condition, body } => self.lower_while(line, condition, body),
            StmtKind::Break => {
                if self.state.loop_depth == 0 {
                    return Err(self.diagnostic(line, Errors::BreakOutsideLoop.into()));
                }
                self.release_for_break(line)?;
                emit::bare(&mut self.state.output, MessageTag::Break);
                Ok(())
            }
            StmtKind::Init(body) => self.lower_init(line, body),
            StmtKind::Expr(expression) => self.lower_expression_statement(line, expression),
            StmtKind::Throw(message) => {
                self.lower_throw(message);
                Ok(())
            }
            StmtKind::Return(value) => self
                .lower_function_return(value.as_ref())
                .map_err(|error| self.diagnostic(line, error)),
        }
    }
}
