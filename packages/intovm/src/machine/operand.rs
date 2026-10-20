use errors::{Errors, Result};
use native::{NativeArg, Operand};

use super::arith::constant;
use super::vm::Vm;
use crate::value::Value;

impl Vm {
    pub(super) fn load_slot(&self, name: &str) -> Result<Value> {
        let frame = self
            .frames
            .last()
            .ok_or_else(|| errors::Error::from(Errors::NoFrame))?;
        let found = if frame.shares(name) {
            self.globals.get(name)
        } else {
            frame.locals.get(name)
        };
        found.cloned().ok_or_else(|| {
            errors::Error::from(Errors::NativeValueNoStorage {
                name: name.to_string(),
            })
        })
    }

    pub(super) fn store(&mut self, name: &str, value: Value) {
        let Some(frame) = self.frames.last_mut() else {
            return;
        };
        if frame.shares(name) {
            self.globals.insert(name.to_owned(), value);
        } else {
            frame.locals.insert(name.to_owned(), value);
        }
    }

    pub(super) fn operand(&self, operand: &Operand, ty: &str) -> Result<Value> {
        match operand {
            Operand::Slot(name) => self.load_slot(name),
            Operand::Str(bytes) => Ok(Value::Str(bytes.clone())),
            Operand::Number(text) => constant(ty, text),
        }
    }

    pub(super) fn argument(&self, argument: &NativeArg) -> Result<Value> {
        match argument {
            NativeArg::Int(number) => Ok(Value::Int(i64::from(*number))),
            NativeArg::Str(bytes) => Ok(Value::Str(bytes.clone())),
            NativeArg::Slot(name) => self.load_slot(name),
            NativeArg::Field { slot, index, .. } => {
                let holder = self.load_slot(slot)?;
                if self.class_of(&holder).is_err() {
                    return Err(errors::Error::from(Errors::NoInstance {
                        slot: slot.to_string(),
                    }));
                }
                self.read_instance_field(&holder, *index)
            }
        }
    }
}
