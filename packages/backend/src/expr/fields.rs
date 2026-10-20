use ast::{Expr, Path};
use errors::{Errors, Result};
use native::FieldLoad;
use types::{Literal, Type};

use super::type_name::{base_type_name, parse_type};
use crate::collect::ancestors;
use crate::context::{Lowerer, Variable};
use crate::emit;

impl Lowerer<'_> {
    /// Whether `class`, or a class it inherits from, is marked `#[dynamic]`.
    pub fn is_dynamic(&self, class: &str) -> bool {
        let dynamic = &self.env.dynamic_classes;
        dynamic.contains(class)
            || ancestors(&self.env.classes, class).is_ok_and(|chain| {
                chain
                    .iter()
                    .any(|ancestor| dynamic.contains(&ancestor.name))
            })
    }

    /// Whether the first member of `path` is one that the dynamic value at
    /// its start does not declare.
    pub fn is_dynamic_member(&self, path: &Path) -> bool {
        let [holder, member, ..] = path.segments() else {
            return false;
        };
        let Some(variable) = self.state.values.get(holder).filter(|v| v.native) else {
            return false;
        };
        let class = base_type_name(&variable.ty.name()).to_owned();
        self.is_dynamic(&class)
            && !self.env.class_fields.get(&class).is_some_and(|fields| {
                fields
                    .iter()
                    .any(|field| !field.is_static && field.name == *member)
            })
    }

    /// Read the member `name` that the dynamic value in `holder` does not
    /// declare, with its `get` method.
    pub(crate) fn read_dynamic_member(
        &mut self,
        holder: &str,
        name: &str,
    ) -> Result<(String, Type)> {
        let call = Expr::Call {
            callee: Path::from(vec![holder.to_owned(), "get".to_owned()]),
            args: vec![Expr::Literal(Literal::String(name.to_owned()))],
        };
        let env = self.env;
        let target = self.fresh_slot(&Type::Pointer);
        let (_, ty, native) = self.declaration_value(&call, None, &env.type_params, &target)?;
        if !native {
            return Err(Errors::DynamicGetNoValue {
                holder: holder.to_string(),
            }
            .into());
        }
        self.state
            .values
            .insert(target.clone(), Variable::temporary(ty.clone()));
        Ok((target, ty))
    }

    /// Read `holder.field`, a field of the instance a binding holds, into a
    /// new slot; `holder.a.b` reads the field `b` of the instance in the field
    /// `a`. Returns `None` when the path does not start with a field.
    pub fn lower_field_read(&mut self, path: &Path) -> Result<Option<(String, Type)>> {
        let [holder, fields @ ..] = path.segments() else {
            return Ok(None);
        };
        if fields.is_empty() {
            return Ok(None);
        }
        if let Some(described) = self.lower_type_property(path)? {
            return Ok(Some(described));
        }
        let Some(variable) = self.state.values.get(holder) else {
            return Ok(None);
        };
        if !variable.native {
            return Ok(None);
        }
        let mut current = (holder.clone(), variable.ty.clone());
        for (position, field) in fields.iter().enumerate() {
            let (slot, receiver_type) = current;
            let class = base_type_name(&receiver_type.name()).to_owned();
            let env = self.env;
            let Some((index, definition)) = env.class_fields.get(&class).and_then(|declared| {
                declared
                    .iter()
                    .filter(|definition| !definition.is_static)
                    .enumerate()
                    .find(|(_, definition)| definition.name == *field)
            }) else {
                if self.is_dynamic(&class) {
                    let (target, ty) = self.read_dynamic_member(&slot, field)?;
                    current = (target, ty);
                    continue;
                }
                if position == 0 {
                    return Ok(None);
                }
                return Err(Errors::NoField {
                    class: class.to_string(),
                    field: field.to_string(),
                }
                .into());
            };
            self.ensure_usable(holder)?;
            let written = parse_type(&definition.ty, &env.type_params)?;
            let ty = self.bind_arguments(&receiver_type, written);
            let target = self.fresh_slot(&ty);
            emit::field_load(
                &mut self.state.output,
                &FieldLoad {
                    target: target.clone(),
                    slot,
                    class,
                    index,
                    ty: ty.name(),
                },
            );
            current = (target, ty);
        }
        Ok(Some(current))
    }
}
