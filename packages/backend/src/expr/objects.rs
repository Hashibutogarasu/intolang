use ast::{Expr, Path, TypeRef};
use types::{Literal, Type};

use crate::context::{Lowerer, Variable};

/// The class an object literal builds, and the static method that makes an
/// empty one.
const JSON_CLASS: &str = "Json";
const NEW_OBJECT: &str = "object";

impl Lowerer<'_> {
    /// The `Json` method that stores a value of type `ty` in an object.
    fn json_setter(ty: &Type) -> Option<&'static str> {
        match ty {
            Type::String | Type::Path => Some("setString"),
            Type::Boolean => Some("setBool"),
            ty if ty.is_integer() || *ty == Type::Char => Some("setInt"),
            ty if ty.is_float() => Some("setFloat"),
            ty if ty.class_name() == Some(JSON_CLASS) => Some("setJson"),
            Type::List(_) => Some("setList"),
            _ => None,
        }
    }

    /// Lower an object literal to a new `Json` object held by the slot
    /// `assigned_to`, with an entry stored for each key.
    pub fn lower_object(
        &mut self,
        entries: &[(String, Expr)],
        assigned_to: &str,
    ) -> Result<Type, String> {
        let env = self.env;
        if !env.class_fields.contains_key(JSON_CLASS) {
            return Err("An object literal needs the class `Json`".to_owned());
        }
        let json = Type::Named(JSON_CLASS.to_owned());
        let make = Expr::Call {
            callee: Path::from(vec![JSON_CLASS.to_owned(), NEW_OBJECT.to_owned()]),
            args: Vec::new(),
        };
        let (_, made, native) =
            self.declaration_value(&make, None, &env.type_params, assigned_to)?;
        if !native {
            return Err("`Json.object` has no value".to_owned());
        }
        self.state
            .values
            .insert(assigned_to.to_owned(), Variable::temporary(made));
        for (key, value) in entries {
            let value = self.object_entry(value)?;
            let ty = self.type_of(&value)?;
            let setter = Self::json_setter(ty.referent()).ok_or_else(|| {
                format!("The value `{value}` of the entry `{key}` cannot be stored in an object")
            })?;
            let value = if ty == Type::Char {
                Expr::Cast {
                    value: Box::new(value),
                    ty: TypeRef::from("Int64"),
                }
            } else {
                value
            };
            let call = Path::from(vec![assigned_to.to_owned(), setter.to_owned()]);
            let args = [Expr::Literal(Literal::String(key.clone())), value];
            self.inline_call(&call, &args, 0)?;
        }
        Ok(json)
    }

    /// An expression a setter can take for the value of an entry: a nested
    /// object or array is built into a slot first.
    fn object_entry(&mut self, value: &Expr) -> Result<Expr, String> {
        if !matches!(value, Expr::Object(_) | Expr::Array(_)) {
            return Ok(value.clone());
        }
        let env = self.env;
        let slot = self.fresh_slot(&Type::Pointer);
        let (_, ty, _) = self.declaration_value(value, None, &env.type_params, &slot)?;
        self.state
            .values
            .insert(slot.clone(), Variable::temporary(ty));
        Ok(Expr::ident(slot))
    }
}
