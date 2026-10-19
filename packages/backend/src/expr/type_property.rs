use ast::{Expr, Path};
use native::{FieldAssign, Operand};
use types::Type;

use super::type_name::base_type_name;
use crate::context::Lowerer;
use crate::emit;

/// The class whose instances describe a type.
pub const TYPE_CLASS: &str = "Type";

/// The property that gives the type of a value.
pub const TYPE_PROPERTY: &str = "type";

impl Lowerer<'_> {
    /// The static type of the value `path` without its last segment names.
    fn type_before_last(&self, path: &Path) -> Option<Type> {
        let segments = path.segments();
        let (_, prefix) = segments.split_last()?;
        if prefix.is_empty() {
            return None;
        }
        if let [name] = prefix {
            let variable = self.state.values.get(name)?;
            return Some(if variable.native {
                variable.ty.referent().clone()
            } else {
                Type::String
            });
        }
        self.type_of(&Expr::Path(Path::from(prefix.to_vec())))
            .ok()
            .map(|ty| ty.referent().clone())
    }

    /// Whether `path` is `value.type` where the class of `value` has no field
    /// called `type`.
    pub fn is_type_property(&self, path: &Path) -> bool {
        if path.last() != TYPE_PROPERTY || !self.env.class_fields.contains_key(TYPE_CLASS) {
            return false;
        }
        let Some(ty) = self.type_before_last(path) else {
            return false;
        };
        let declares = self
            .env
            .class_fields
            .get(base_type_name(&ty.name()))
            .is_some_and(|fields| {
                fields
                    .iter()
                    .any(|field| !field.is_static && field.name == TYPE_PROPERTY)
            });
        !declares
    }

    /// Lower `value.type` to a new instance of `Type` that holds the name of
    /// the type of `value`.
    pub fn lower_type_property(&mut self, path: &Path) -> Result<Option<(String, Type)>, String> {
        if !self.is_type_property(path) {
            return Ok(None);
        }
        let Some(described) = self.type_before_last(path) else {
            return Ok(None);
        };
        let env = self.env;
        let index = env
            .class_fields
            .get(TYPE_CLASS)
            .and_then(|fields| {
                fields
                    .iter()
                    .filter(|field| !field.is_static)
                    .position(|field| field.name == "name")
            })
            .ok_or_else(|| "The class `Type` has no field `name`".to_owned())?;
        let result = Type::Named(TYPE_CLASS.to_owned());
        let slot = self.fresh_slot(&result);
        emit::construct(&mut self.state.output, &slot, TYPE_CLASS);
        emit::field_assign(
            &mut self.state.output,
            &FieldAssign {
                slot: slot.clone(),
                class: TYPE_CLASS.to_owned(),
                index,
                ty: Type::String.name(),
                operand: Operand::Str(described.name().into_bytes()),
            },
        );
        Ok(Some((slot, result)))
    }
}
