use ast::{Path, Stmt};
use errors::Result;
use types::MessageTag;

use crate::context::{Lowerer, Ownership};
use crate::emit;
use crate::expr::{DESTROY, base_type_name};

impl Lowerer<'_> {
    /// Lower `body` as a scope: every binding it declares and still owns when
    /// control leaves it is destroyed, last declared first.
    pub fn lower_scope(&mut self, line: usize, body: &[Stmt]) -> Result<()> {
        self.state.scopes.push(Vec::new());
        let lowered = self.lower_block(body);
        let names = self.state.scopes.pop().unwrap_or_default();
        lowered?;
        self.release_scope(&names, None)
            .map_err(|error| self.diagnostic(line, error))
    }

    /// Destroy what the outermost scope still owns once the program ends.
    pub fn finish_program(&mut self) -> Result<()> {
        let names = self.state.scopes.pop().unwrap_or_default();
        self.release_scope(&names, None)
            .map_err(|error| self.diagnostic(0, error))
    }

    /// Destroy what the scopes between a `break` and its loop own, without
    /// changing what the code after the `break` sees.
    pub fn release_for_break(&mut self, line: usize) -> Result<()> {
        let Some(&first) = self.state.loop_scopes.last() else {
            return Ok(());
        };
        let snapshot = self.state.values.clone();
        let scopes = self.state.scopes[first..].to_vec();
        let released = scopes
            .iter()
            .rev()
            .try_for_each(|names| self.release_scope(names, None));
        self.state.values = snapshot;
        released.map_err(|error| self.diagnostic(line, error))
    }

    /// Start tracking a binding that was just declared in the current scope.
    pub fn declare_in_scope(&mut self, name: &str) {
        if !self.is_object_binding(name) {
            return;
        }
        if let Some(scope) = self.state.scopes.last_mut() {
            scope.retain(|declared| declared != name);
            scope.push(name.to_owned());
        }
        if self.owns_instance(name) {
            emit::drop_flag(&mut self.state.output, name, true);
        }
    }

    /// Whether `name` was declared by a scope enclosing the current one.
    pub fn declared_in_outer_scope(&self, name: &str) -> bool {
        let Some((_, outer)) = self.state.scopes.split_last() else {
            return false;
        };
        outer
            .iter()
            .any(|names| names.iter().any(|declared| declared == name))
    }

    /// Whether `name` holds an instance of a class that inherits `Object`.
    fn is_object_binding(&self, name: &str) -> bool {
        self.state.values.get(name).is_some_and(|variable| {
            variable.native
                && self.is_ownership_method(base_type_name(&variable.ty.name()), DESTROY)
        })
    }

    /// Whether `name` owns its instance, as opposed to referring to one that
    /// another binding owns.
    pub fn owns_instance(&self, name: &str) -> bool {
        self.is_object_binding(name)
            && self
                .state
                .values
                .get(name)
                .is_some_and(|variable| !variable.tracking.reference && !variable.unloaded)
    }

    /// Release the bindings of one scope in reverse declaration order, except
    /// `keep`, which the scope hands to its caller.
    pub fn release_scope(&mut self, names: &[String], keep: Option<&str>) -> Result<()> {
        names
            .iter()
            .rev()
            .filter(|name| Some(name.as_str()) != keep)
            .try_for_each(|name| self.release_binding(name))
    }

    fn release_binding(&mut self, name: &str) -> Result<()> {
        let Some(variable) = self.state.values.get(name) else {
            return Ok(());
        };
        if variable.unloaded {
            return Ok(());
        }
        let tracking = variable.tracking.clone();
        if tracking.reference {
            if tracking.state == Ownership::Live {
                self.release_borrow(name);
                self.set_state(name, Ownership::Destroyed);
            }
            return Ok(());
        }
        match (tracking.state, tracking.conditional) {
            (Ownership::Live, _) => self.destroy_binding(name),
            (_, true) => {
                emit::drop_guard(&mut self.state.output, name);
                self.restore_state(name, Ownership::Live);
                let destroyed = self.destroy_binding(name);
                emit::bare(&mut self.state.output, MessageTag::IfEnd);
                destroyed
            }
            _ => Ok(()),
        }
    }

    fn restore_state(&mut self, name: &str, state: Ownership) {
        if let Some(variable) = self.state.values.get_mut(name) {
            variable.tracking.state = state;
        }
    }

    fn destroy_binding(&mut self, name: &str) -> Result<()> {
        let callee = Path::from(vec![name.to_owned(), DESTROY.to_owned()]);
        self.inline_call(&callee, &[], 0)
    }
}
