use anyhow::{Result, anyhow};
use native::{OwnershipChange, Property, StateCheck};
use types::Storage;

use super::vm::Vm;

/// What has happened to the instance a binding holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Live,
    Moved,
    Destroyed,
    Detached,
}

/// A binding the program declared: how it is stored, whether its memory
/// exists, and who owns its instance.
#[derive(Debug, Clone)]
pub struct Binding {
    pub storage: Storage,
    pub allocated: bool,
    pub state: State,
    pub reference: bool,
    pub borrowed_by: usize,
    pub borrows: Option<String>,
}

impl Binding {
    fn owned(storage: Storage) -> Self {
        Self {
            storage,
            allocated: true,
            state: State::Live,
            reference: false,
            borrowed_by: 0,
            borrows: None,
        }
    }

    fn declared(storage: Storage) -> Self {
        Self {
            allocated: storage == Storage::Now,
            ..Self::owned(storage)
        }
    }
}

impl Vm {
    pub fn binding(&self, name: &str) -> Option<&Binding> {
        self.bindings.get(name)
    }

    fn binding_mut(&mut self, name: &str) -> Result<&mut Binding> {
        self.bindings
            .get_mut(name)
            .ok_or_else(|| anyhow!("`{name}` has no binding"))
    }

    pub(super) fn apply(&mut self, change: &OwnershipChange) -> Result<()> {
        match change {
            OwnershipChange::Declare { name, storage } => match self.bindings.get_mut(name) {
                Some(binding) => binding.storage = *storage,
                None => {
                    self.bindings
                        .insert(name.clone(), Binding::declared(*storage));
                }
            },
            OwnershipChange::Init(started) => self.in_init = *started,
            OwnershipChange::Copy { target, source } => {
                self.binding_mut(source)?;
                self.bindings
                    .insert(target.clone(), Binding::owned(Storage::Local));
            }
            OwnershipChange::Move { target, source } => {
                let source = self.binding_mut(source)?;
                source.state = State::Moved;
                source.allocated = false;
                self.bindings
                    .insert(target.clone(), Binding::owned(Storage::Local));
            }
            OwnershipChange::Borrow { target, source } => {
                self.binding_mut(source)?.borrowed_by += 1;
                let mut binding = Binding::owned(Storage::Local);
                binding.reference = true;
                binding.borrows = Some(source.clone());
                self.bindings.insert(target.clone(), binding);
            }
            OwnershipChange::Moved(name) => {
                let binding = self.binding_mut(name)?;
                binding.state = State::Moved;
                binding.allocated = false;
            }
            OwnershipChange::Destroyed(name) => {
                let binding = self.binding_mut(name)?;
                binding.state = State::Destroyed;
                binding.allocated = false;
                let owner = binding.borrows.take();
                if let Some(owner) = owner.and_then(|owner| self.bindings.get_mut(&owner)) {
                    owner.borrowed_by = owner.borrowed_by.saturating_sub(1);
                }
            }
            OwnershipChange::Detached(name) => {
                let binding = self.binding_mut(name)?;
                binding.state = State::Detached;
                binding.allocated = false;
            }
            OwnershipChange::Unload(name) => self.binding_mut(name)?.allocated = false,
        }
        Ok(())
    }

    pub(super) fn check_state(&mut self, check: &StateCheck) -> Result<bool> {
        let in_init = self.in_init;
        let binding = self.binding_mut(&check.slot)?;
        let value = match check.property {
            Property::InMemory => {
                if binding.state != State::Live {
                    false
                } else {
                    match binding.storage {
                        Storage::Now => binding.allocated,
                        Storage::Late => {
                            binding.allocated = true;
                            true
                        }
                        Storage::Local => in_init,
                    }
                }
            }
            Property::Owned => binding.state == State::Live && !binding.reference,
            Property::Reference => binding.reference,
            Property::Borrowed => binding.borrowed_by > 0,
            Property::Moved => binding.state == State::Moved,
            Property::Destroyed => binding.state == State::Destroyed,
        };
        Ok(value == check.expected)
    }
}
