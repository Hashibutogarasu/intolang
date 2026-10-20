use ast::{Stmt, StmtKind};
use errors::{Errors, Result};
use types::MessageTag;

use crate::context::Lowerer;
use crate::emit;

impl Lowerer<'_> {
    pub fn lower_block(&mut self, body: &[Stmt]) -> Result<()> {
        body.iter().try_for_each(|stmt| self.lower_stmt(stmt))
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
