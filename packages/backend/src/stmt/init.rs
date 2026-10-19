use anyhow::Result;
use ast::Stmt;
use native::OwnershipChange;

use crate::context::Lowerer;
use crate::emit;

impl Lowerer<'_> {
    /// Lower an `init { ... }` entry block; bindings declared inside it are
    /// only visible there.
    pub fn lower_init(&mut self, line: usize, body: &[Stmt]) -> Result<()> {
        let outer = self.state.in_init;
        self.state.in_init = true;
        self.state.init_has_run = true;
        emit::ownership(&mut self.state.output, &OwnershipChange::Init(true));
        let lowered = self.lower_scope(line, body);
        emit::ownership(&mut self.state.output, &OwnershipChange::Init(outer));
        self.state.in_init = outer;
        lowered
    }
}
