use types::Type;

use super::type_name::base_type_name;
use crate::collect::ancestors;
use crate::context::{Lowerer, OBJECT, Ownership, Tracking};
use crate::emit;

/// The `Object` members that change what a binding owns.
pub const COPY: &str = "copy";
pub const MOVE: &str = "move";
pub const BORROW: &str = "borrow";
pub const DESTROY: &str = "destroy";

impl Lowerer<'_> {
    /// Fail when `name` no longer holds a usable instance.
    pub fn ensure_usable(&self, name: &str) -> Result<(), String> {
        let Some(variable) = self.state.values.get(name) else {
            return Ok(());
        };
        match variable.tracking.state {
            Ownership::Live => Ok(()),
            Ownership::Moved => Err(format!("`{name}` was moved")),
            Ownership::Destroyed => Err(format!("`{name}` was destroyed")),
        }
    }

    /// Fail when a binding that others borrow from would give up its instance.
    fn ensure_not_borrowed(&self, name: &str) -> Result<(), String> {
        match self.state.values.get(name) {
            Some(variable) if variable.tracking.borrowed_by > 0 => {
                Err(format!("`{name}` is borrowed"))
            }
            _ => Ok(()),
        }
    }

    /// Whether `method` on a receiver of type `class` is one of the built-in
    /// `Object` members that ownership tracking must see, as opposed to a
    /// member the class or a parent declares itself.
    pub fn is_ownership_method(&self, class: &str, method: &str) -> bool {
        matches!(method, COPY | MOVE | BORROW | DESTROY)
            && ancestors(&self.env.classes, class)
                .is_ok_and(|chain| chain.iter().any(|ancestor| ancestor.name == OBJECT))
    }

    /// Check a method call on the binding `receiver` before it is lowered.
    pub fn check_receiver_call(&self, receiver: &str, method: &str) -> Result<(), String> {
        self.ensure_usable(receiver)?;
        if matches!(method, MOVE | DESTROY) {
            self.ensure_not_borrowed(receiver)?;
        }
        Ok(())
    }

    /// Record what a lowered method call did to the binding `receiver`.
    pub fn apply_receiver_call(&mut self, receiver: &str, method: &str) {
        match method {
            MOVE => self.set_state(receiver, Ownership::Moved),
            DESTROY => {
                self.set_state(receiver, Ownership::Destroyed);
                self.release_borrow(receiver);
            }
            _ => {}
        }
    }

    /// Record that `name` gave up its instance. When that happens in a scope
    /// nested inside the one that declared `name`, the scope end cannot know
    /// statically whether the instance is still owned, so the drop flag is
    /// cleared at run time.
    pub fn set_state(&mut self, name: &str, state: Ownership) {
        let conditional = self.declared_in_outer_scope(name) && self.owns_instance(name);
        if let Some(variable) = self.state.values.get_mut(name) {
            variable.tracking.state = state;
            variable.tracking.conditional |= conditional;
        }
        if conditional {
            emit::drop_flag(&mut self.state.output, name, false);
        }
    }

    /// Give back the borrow `name` holds on its owner, if it is a borrow.
    pub fn release_borrow(&mut self, name: &str) {
        let owner = self
            .state
            .values
            .get(name)
            .and_then(|variable| variable.tracking.borrows.clone());
        if let Some(owner) = owner
            && let Some(variable) = self.state.values.get_mut(&owner)
        {
            variable.tracking.borrowed_by = variable.tracking.borrowed_by.saturating_sub(1);
        }
    }

    /// Lower `receiver.copy()`, `receiver.move()` or `receiver.borrow()` as
    /// the value of `assigned_to`, returning the type the new binding has.
    pub fn lower_ownership_value(
        &mut self,
        receiver: &str,
        receiver_type: &Type,
        method: &str,
        assigned_to: &str,
    ) -> Result<Type, String> {
        self.check_receiver_call(receiver, method)?;
        let class = base_type_name(&receiver_type.name()).to_owned();
        let mut tracking = Tracking::default();
        match method {
            COPY => {
                emit::copy(&mut self.state.output, assigned_to, receiver, &class);
            }
            MOVE => {
                emit::alias(&mut self.state.output, assigned_to, receiver);
                self.set_state(receiver, Ownership::Moved);
            }
            BORROW => {
                emit::alias(&mut self.state.output, assigned_to, receiver);
                tracking.borrows = Some(receiver.to_owned());
                tracking.reference = true;
                if let Some(owner) = self.state.values.get_mut(receiver) {
                    owner.tracking.borrowed_by += 1;
                }
            }
            other => return Err(format!("`{other}` does not produce a value")),
        }
        self.state.pending_tracking = Some(tracking);
        Ok(receiver_type.clone())
    }
}
