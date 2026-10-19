use anyhow::Result;
use ast::{Expr, Path};
use native::OwnershipChange;

use crate::context::Lowerer;

impl Lowerer<'_> {
    /// Lower the reassignment of an existing binding: `name = value;`.
    /// Assignments to unknown names produce no output.
    pub fn lower_assignment(&mut self, line: usize, target: &Path, value: &Expr) -> Result<()> {
        let env = self.env;
        let Some(name) = target.as_ident() else {
            return Ok(());
        };
        let Some(current) = self.state.values.get(name) else {
            return Ok(());
        };
        let target_type = current.ty.clone();
        let (text, ty, native) = self
            .declaration_value(value, Some(target_type), &env.type_params, name)
            .map_err(|message| self.diagnostic(line, &message))?;
        if let Some(variable) = self.state.values.get_mut(name) {
            variable.value = text;
            variable.ty = ty;
            variable.native = native;
            variable.unloaded = false;
            variable.allocated = true;
        }
        Ok(())
    }

    /// Mark a memory backed binding as unloaded: `unload name;`.
    pub fn lower_unload(&mut self, name: &str) {
        if let Some(variable) = self.state.values.get_mut(name) {
            variable.allocated = false;
            variable.unloaded = true;
            self.emit_change(OwnershipChange::Unload(name.to_owned()));
        }
    }
}
