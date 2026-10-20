use ast::{Expr, Path};
use errors::{Errors, Result};
use native::{CastAssign, FieldAssign, Operand, OwnershipChange};
use parser::Hex;
use types::Type;

use crate::context::Lowerer;
use crate::emit;
use crate::expr::{base_type_name, parse_type};

impl Lowerer<'_> {
    /// Lower the reassignment of an existing binding, `name = value;`, or of
    /// a field of the instance a binding holds, `name.field = value;`.
    pub fn lower_assignment(&mut self, line: usize, target: &Path, value: &Expr) -> Result<()> {
        if let [holder, field] = target.segments() {
            return self
                .lower_field_assignment(holder, field, value)
                .map_err(|error| self.diagnostic(line, error));
        }
        let env = self.env;
        let Some(name) = target.as_ident() else {
            return Err(self.diagnostic(
                line,
                Errors::AssignTargetInvalid {
                    target: target.to_string(),
                }
                .into(),
            ));
        };
        let Some(current) = self.state.values.get(name) else {
            return Err(self.diagnostic(
                line,
                Errors::UndefinedName {
                    name: name.to_string(),
                }
                .into(),
            ));
        };
        if current.constant {
            return Err(self.diagnostic(
                line,
                Errors::AssignToConstant {
                    name: name.to_string(),
                }
                .into(),
            ));
        }
        let target_type = current.ty.clone();
        let in_slot = current.native;
        let (text, ty, native) = self
            .declaration_value(value, Some(target_type), &env.type_params, name)
            .map_err(|error| self.diagnostic(line, error))?;
        let (text, native) = if in_slot && !native && ty == Type::String {
            let hex = Hex::from(text.as_str()).to_string();
            emit::assign(&mut self.state.output, name, &ty, "s", &hex);
            (ty.name(), true)
        } else {
            (text, native)
        };
        if let Some(variable) = self.state.values.get_mut(name) {
            variable.value = text;
            variable.ty = ty;
            variable.native = native;
            variable.unloaded = false;
            variable.allocated = true;
        }
        Ok(())
    }

    fn lower_field_assignment(&mut self, holder: &str, field: &str, value: &Expr) -> Result<()> {
        let env = self.env;
        self.ensure_usable(holder)?;
        let variable = self.state.values.get(holder).ok_or_else(|| {
            errors::Error::from(Errors::UndefinedName {
                name: holder.to_string(),
            })
        })?;
        if variable.constant {
            return Err(Errors::AssignToConstant {
                name: holder.to_string(),
            }
            .into());
        }
        if !variable.native {
            return Err(Errors::HoldsNoInstance {
                holder: holder.to_string(),
            }
            .into());
        }
        let receiver_type = variable.ty.clone();
        let class = base_type_name(&receiver_type.name()).to_owned();
        let fields = env.class_fields.get(&class).ok_or_else(|| {
            errors::Error::from(Errors::NoField {
                class: class.clone(),
                field: field.to_string(),
            })
        })?;
        let (index, definition) = fields
            .iter()
            .filter(|definition| !definition.is_static)
            .enumerate()
            .find(|(_, definition)| definition.name == field)
            .ok_or_else(|| {
                errors::Error::from(Errors::NoField {
                    class: class.clone(),
                    field: field.to_string(),
                })
            })?;
        let written = parse_type(&definition.ty, &env.type_params)?;
        let field_type = self.bind_arguments(&receiver_type, written);
        let operand = self.field_operand(field, &field_type, value)?;
        emit::field_assign(
            &mut self.state.output,
            &FieldAssign {
                slot: holder.to_owned(),
                class,
                index,
                ty: field_type.name(),
                operand,
            },
        );
        Ok(())
    }

    /// The operand that stores `value` in a field that holds a list, an
    /// instance or a pointer: the slot of a binding, or a new one that the
    /// value is built into.
    fn reference_operand(
        &mut self,
        field: &str,
        field_type: &Type,
        value: &Expr,
    ) -> Result<Operand> {
        if let Some(name) = value.as_ident() {
            self.ensure_usable(name)?;
            let held = self
                .state
                .values
                .get(name)
                .filter(|variable| variable.native)
                .ok_or_else(|| {
                    errors::Error::from(Errors::FieldHoldsNoValue {
                        field: field.to_string(),
                        expected: field_type.name(),
                        name: name.to_string(),
                    })
                })?;
            if !held.ty.is_assignable_to(field_type) {
                return Err(Errors::FieldReceivedType {
                    field: field.to_string(),
                    expected: field_type.name(),
                    received: held.ty.name(),
                }
                .into());
            }
            return Ok(Operand::Slot(name.to_owned()));
        }
        if !matches!(
            value,
            Expr::Array(_) | Expr::Object(_) | Expr::Call { .. } | Expr::Path(_)
        ) {
            return Err(Errors::FieldAssignUnsupported {
                value: value.to_string(),
                ty: field_type.name(),
                field: field.to_string(),
            }
            .into());
        }
        let env = self.env;
        let slot = self.fresh_slot(field_type);
        let (_, built, native) =
            self.declaration_value(value, Some(field_type.clone()), &env.type_params, &slot)?;
        if !native {
            return Err(Errors::FieldTypeMismatch {
                field: field.to_string(),
                expected: field_type.name(),
                received: value.to_string(),
            }
            .into());
        }
        self.state
            .values
            .insert(slot.clone(), crate::context::Variable::temporary(built));
        Ok(Operand::Slot(slot))
    }

    /// The operand that stores `value` in a field of type `field_type`.
    fn field_operand(&mut self, field: &str, field_type: &Type, value: &Expr) -> Result<Operand> {
        let mismatch = |received: String| -> errors::Error {
            Errors::FieldTypeMismatch {
                field: field.to_string(),
                expected: field_type.name(),
                received,
            }
            .into()
        };
        match field_type {
            Type::String | Type::Path => {
                if !self.is_text(value) {
                    return Err(mismatch(value.to_string()));
                }
                Ok(match self.lower_text(value, None)? {
                    crate::expr::Text::Const(text) => Operand::Str(text.into_bytes()),
                    crate::expr::Text::Slot(slot) => Operand::Slot(slot),
                })
            }
            ty if ty.is_numeric() || matches!(ty, Type::Boolean | Type::Char) => {
                let declared = self.type_of(value)?;
                if !(declared.is_numeric() || matches!(declared, Type::Boolean | Type::Char)) {
                    return Err(mismatch(declared.name()));
                }
                let numeric = self.lower_numeric(value)?;
                let received = match &numeric.kind {
                    crate::expr::Kind::Typed(received) => received.clone(),
                    crate::expr::Kind::IntLiteral => Type::Int32,
                    crate::expr::Kind::FloatLiteral => Type::Float64,
                };
                let fits = match numeric.kind {
                    crate::expr::Kind::IntLiteral => ty.is_numeric(),
                    crate::expr::Kind::FloatLiteral => ty.is_float(),
                    crate::expr::Kind::Typed(_) => received.is_assignable_to(ty),
                };
                if !fits {
                    return Err(mismatch(received.name()));
                }
                match numeric.operand {
                    Operand::Slot(slot) if received != *ty => {
                        let converted = self.fresh_slot(ty);
                        emit::cast_assign(
                            &mut self.state.output,
                            &CastAssign {
                                target: converted.clone(),
                                source: slot,
                                from: received.name(),
                                to: ty.name(),
                            },
                        );
                        Ok(Operand::Slot(converted))
                    }
                    other => Ok(other),
                }
            }
            reference => self.reference_operand(field, reference, value),
        }
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
