use anyhow::Result;
use ast::{Expr, Path};
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
                .map_err(|message| self.diagnostic(line, &message));
        }
        let env = self.env;
        let Some(name) = target.as_ident() else {
            return Err(self.diagnostic(
                line,
                &format!("Only a name or a field of a name can be assigned, not `{target}`"),
            ));
        };
        let Some(current) = self.state.values.get(name) else {
            return Err(self.diagnostic(line, &format!("Undefined name `{name}`")));
        };
        let target_type = current.ty.clone();
        let in_slot = current.native;
        let (text, ty, native) = self
            .declaration_value(value, Some(target_type), &env.type_params, name)
            .map_err(|message| self.diagnostic(line, &message))?;
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

    fn lower_field_assignment(
        &mut self,
        holder: &str,
        field: &str,
        value: &Expr,
    ) -> Result<(), String> {
        let env = self.env;
        self.ensure_usable(holder)?;
        let variable = self
            .state
            .values
            .get(holder)
            .ok_or_else(|| format!("Undefined name `{holder}`"))?;
        if !variable.native {
            return Err(format!("`{holder}` holds no instance"));
        }
        let class = base_type_name(&variable.ty.name()).to_owned();
        let fields = env
            .class_fields
            .get(&class)
            .ok_or_else(|| format!("`{class}` has no field `{field}`"))?;
        let (index, definition) = fields
            .iter()
            .filter(|definition| !definition.is_static)
            .enumerate()
            .find(|(_, definition)| definition.name == field)
            .ok_or_else(|| format!("`{class}` has no field `{field}`"))?;
        let field_type =
            parse_type(&definition.ty, &env.type_params).map_err(|error| error.to_string())?;
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

    /// The operand that stores `value` in a field of type `field_type`.
    fn field_operand(
        &mut self,
        field: &str,
        field_type: &Type,
        value: &Expr,
    ) -> Result<Operand, String> {
        let mismatch = |received: String| {
            format!(
                "The field `{field}` is a `{}` but received `{received}`",
                field_type.name()
            )
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
            other => Err(format!(
                "Assigning a `{}` field is not supported",
                other.name()
            )),
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
