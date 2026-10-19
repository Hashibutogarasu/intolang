use ast::Path;
use native::FieldLoad;
use types::Type;

use super::type_name::{base_type_name, parse_type};
use crate::context::Lowerer;
use crate::emit;

impl Lowerer<'_> {
    /// Read `holder.field`, a field of the instance a binding holds, into a
    /// new slot; `holder.a.b` reads the field `b` of the instance in the field
    /// `a`. Returns `None` when the path does not start with a field.
    pub fn lower_field_read(&mut self, path: &Path) -> Result<Option<(String, Type)>, String> {
        let [holder, fields @ ..] = path.segments() else {
            return Ok(None);
        };
        if fields.is_empty() {
            return Ok(None);
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
                if position == 0 {
                    return Ok(None);
                }
                return Err(format!("`{class}` has no field `{field}`"));
            };
            self.ensure_usable(holder)?;
            let written =
                parse_type(&definition.ty, &env.type_params).map_err(|error| error.to_string())?;
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
