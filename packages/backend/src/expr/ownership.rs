use errors::{Errors, Result};
use native::OwnershipChange;
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
    pub fn ensure_usable(&self, name: &str) -> Result<()> {
        let Some(variable) = self.state.values.get(name) else {
            return Ok(());
        };
        match variable.tracking.state {
            Ownership::Live => Ok(()),
            Ownership::Moved => Err(Errors::Moved {
                name: name.to_string(),
            }
            .into()),
            Ownership::Destroyed => Err(Errors::Destroyed {
                name: name.to_string(),
            }
            .into()),
            Ownership::Detached => Err(Errors::Detached {
                name: name.to_string(),
            }
            .into()),
        }
    }

    /// Fail when a binding that others borrow from would give up its instance.
    fn ensure_not_borrowed(&self, name: &str) -> Result<()> {
        match self.state.values.get(name) {
            Some(variable) if variable.tracking.borrowed_by > 0 => Err(Errors::Borrowed {
                name: name.to_string(),
            }
            .into()),
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

    /// The tracking of a reference to the instance `owner` holds; the owner,
    /// when it is a binding, counts the borrow.
    pub fn borrow_of(&mut self, owner: &str) -> Tracking {
        let mut tracking = Tracking {
            reference: true,
            ..Tracking::default()
        };
        if let Some(variable) = self.state.values.get_mut(owner) {
            variable.tracking.borrowed_by += 1;
            tracking.borrows = Some(owner.to_owned());
        }
        tracking
    }

    /// Whether `name` refers to an instance another binding owns.
    pub fn is_reference_binding(&self, name: &str) -> bool {
        self.state
            .values
            .get(name)
            .is_some_and(|variable| variable.tracking.reference)
    }

    /// What calling `method` on `receiver` makes of the binding when the call
    /// takes the ownership of its instance.
    fn consumed_state(&self, receiver: &str, method: &str) -> Option<Ownership> {
        if method == MOVE {
            return Some(Ownership::Moved);
        }
        let variable = self.state.values.get(receiver)?;
        let class = base_type_name(&variable.ty.name()).to_owned();
        self.env
            .consuming
            .get(&self.method_key(&class, method))
            .copied()
    }

    fn consumes(&self, receiver: &str, method: &str) -> bool {
        self.consumed_state(receiver, method).is_some()
    }

    /// Check a method call on the binding `receiver` before it is lowered.
    pub fn check_receiver_call(&self, receiver: &str, method: &str) -> Result<()> {
        self.ensure_usable(receiver)?;
        if self.is_reference_binding(receiver) {
            if self.consumes(receiver, method) {
                return Err(Errors::NotOwned {
                    receiver: receiver.to_string(),
                    method: method.to_string(),
                }
                .into());
            }
            return Ok(());
        }
        if method == DESTROY || self.consumes(receiver, method) {
            self.ensure_not_borrowed(receiver)?;
        }
        Ok(())
    }

    /// `destroy()` on a reference only gives the reference back and leaves the
    /// instance to its owner. Returns whether the call was handled that way.
    pub fn give_back_reference(&mut self, receiver: &str, method: &str) -> bool {
        if method != DESTROY || !self.is_reference_binding(receiver) {
            return false;
        }
        self.apply_receiver_call(receiver, method);
        true
    }

    /// Record what a lowered method call did to the binding `receiver`.
    pub fn apply_receiver_call(&mut self, receiver: &str, method: &str) {
        match method {
            MOVE => self.set_state(receiver, Ownership::Moved),
            DESTROY => {
                self.set_state(receiver, Ownership::Destroyed);
                self.release_borrow(receiver);
            }
            _ => {
                if let Some(state) = self.consumed_state(receiver, method) {
                    self.set_state(receiver, state);
                }
            }
        }
    }

    /// Record that `name` gave up its instance. When that happens in a scope
    /// nested inside the one that declared `name`, the scope end cannot know
    /// statically whether the instance is still owned, so the drop flag is
    /// cleared at run time.
    pub fn set_state(&mut self, name: &str, state: Ownership) {
        match state {
            Ownership::Live => {}
            Ownership::Moved => self.emit_change(OwnershipChange::Moved(name.to_owned())),
            Ownership::Destroyed => self.emit_change(OwnershipChange::Destroyed(name.to_owned())),
            Ownership::Detached => self.emit_change(OwnershipChange::Detached(name.to_owned())),
        }
        let conditional = self.declared_in_outer_scope(name) && self.owns_instance(name);
        if let Some(variable) = self.state.values.get_mut(name) {
            variable.tracking.state = state;
            variable.tracking.conditional |= conditional;
        }
        if conditional {
            emit::drop_flag(&mut self.state.output, name, false);
        }
    }

    pub fn emit_change(&mut self, change: OwnershipChange) {
        emit::ownership(&mut self.state.output, &change);
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
    ) -> Result<Type> {
        self.check_receiver_call(receiver, method)?;
        let class = base_type_name(&receiver_type.name()).to_owned();
        let referent = receiver_type.referent().clone();
        let mut tracking = Tracking::default();
        match method {
            COPY => {
                emit::copy(&mut self.state.output, assigned_to, receiver, &class);
                self.emit_change(OwnershipChange::Copy {
                    target: assigned_to.to_owned(),
                    source: receiver.to_owned(),
                });
            }
            MOVE => {
                emit::alias(&mut self.state.output, assigned_to, receiver);
                self.emit_change(OwnershipChange::Move {
                    target: assigned_to.to_owned(),
                    source: receiver.to_owned(),
                });
                self.set_state(receiver, Ownership::Moved);
            }
            BORROW => {
                emit::alias(&mut self.state.output, assigned_to, receiver);
                self.emit_change(OwnershipChange::Borrow {
                    target: assigned_to.to_owned(),
                    source: receiver.to_owned(),
                });
                tracking.borrows = Some(receiver.to_owned());
                tracking.reference = true;
                if let Some(owner) = self.state.values.get_mut(receiver) {
                    owner.tracking.borrowed_by += 1;
                }
            }
            other => {
                return Err(Errors::ProducesNoValue {
                    value: other.to_string(),
                }
                .into());
            }
        }
        self.state.pending_tracking = Some(tracking);
        Ok(if method == BORROW {
            Type::Reference(Box::new(referent))
        } else {
            referent
        })
    }
}
