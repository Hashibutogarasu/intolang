use anyhow::Result;
use ast::Stmt;

use crate::context::Lowerer;

impl Lowerer<'_> {
    /// Lower an `init { ... }` entry block; bindings declared inside it are
    /// only visible there.
    pub fn lower_init(&mut self, line: usize, body: &[Stmt]) -> Result<()> {
        let outer = self.state.in_init;
        self.state.in_init = true;
        self.state.init_has_run = true;
        let lowered = self.lower_scope(line, body);
        self.state.in_init = outer;
        lowered
    }
}
