use ast::Expr;
use errors::{Errors, Result};
use std::collections::HashSet;
use types::Type;

use super::numeric::Kind;
use super::type_name::parse_type;
use crate::collect::{CAST_EVENT, ancestors, cast_handler_key};
use crate::context::{Lowerer, Variable};
use crate::emit;

impl Lowerer<'_> {
    /// Whether `ty` names a class declared in the program or built in.
    pub fn is_class_type(&self, ty: &Type) -> bool {
        ty.class_name()
            .is_some_and(|name| self.env.classes.contains_key(name))
    }

    /// Lower `value as class`: build the instance and run the `onCast`
    /// handler that the class, or one of its ancestors, declares for the type
    /// of `value`. The instance ends up in `assigned_to`.
    pub fn lower_class_cast(
        &mut self,
        value: &Expr,
        class: &str,
        assigned_to: &str,
    ) -> Result<Type> {
        let (source, source_type) = self.cast_source(value)?;
        let key = self
            .find_cast_handler(&source_type.name(), class)
            .ok_or_else(|| {
                errors::Error::from(Errors::NoCast {
                    from: source_type.name(),
                    to: class.to_string(),
                })
            })?;
        let ty = Type::Named(class.to_owned());
        emit::construct(&mut self.state.output, assigned_to, class);
        self.state
            .values
            .insert(assigned_to.to_owned(), Variable::temporary(ty.clone()));
        let env = self.env;
        let handler = env.user_functions.get(&key).ok_or_else(|| {
            errors::Error::from(Errors::UndefinedCastHandler { key: key.clone() })
        })?;
        self.inline_user_function(handler, None, &[source, Expr::ident(assigned_to)], 0)?;
        Ok(ty)
    }

    /// The expression that stands for the value being cast, and its type. A
    /// scalar is computed into a slot.
    fn cast_source(&mut self, value: &Expr) -> Result<(Expr, Type)> {
        let declared = self.type_of(value)?;
        if declared.is_numeric() || matches!(declared, Type::Char | Type::Boolean) {
            let numeric = self.lower_numeric(value)?;
            let ty = match &numeric.kind {
                Kind::Typed(ty) => ty.clone(),
                Kind::IntLiteral => Type::Int32,
                Kind::FloatLiteral => Type::Float64,
            };
            let slot = self.slot_of(&numeric, &ty);
            return Ok((Expr::ident(slot), ty));
        }
        if value.as_ident().is_some() {
            return Ok((value.clone(), declared));
        }
        Err(Errors::CannotBeCast {
            value: value.to_string(),
        }
        .into())
    }

    /// The handler for a cast from `from` to `class`: the concrete
    /// `onCast<from, class>` first, then a generic `onCast<From, To>`, in the
    /// class and then in each ancestor.
    fn find_cast_handler(&self, from: &str, class: &str) -> Option<String> {
        let mut chain = vec![class.to_owned()];
        chain.extend(
            ancestors(&self.env.classes, class)
                .ok()?
                .iter()
                .map(|ancestor| ancestor.name.clone()),
        );
        for owner in chain {
            let handlers = self.env.classes.get(&owner)?.events.iter().filter(|event| {
                event.name == CAST_EVENT && event.has_body && event.type_params.len() == 2
            });
            let key = |event: &crate::context::MemberInfo| {
                cast_handler_key(&owner, &event.type_params[0], &event.type_params[1])
            };
            let declared = |event: &crate::context::MemberInfo| {
                self.env.user_functions.contains_key(&key(event))
            };
            let exact = handlers.clone().find(|event| {
                declared(event)
                    && event.type_params[0] == from
                    && (event.type_params[1] == class || event.type_params[1] == owner)
            });
            let generic = || {
                handlers.clone().find(|event| {
                    declared(event)
                        && !self.names_a_type(&event.type_params[0])
                        && !self.names_a_type(&event.type_params[1])
                })
            };
            if let Some(event) = exact.or_else(generic) {
                return Some(key(event));
            }
        }
        None
    }

    fn names_a_type(&self, name: &str) -> bool {
        match parse_type(name, &HashSet::new()) {
            Ok(Type::Named(name)) => self.env.classes.contains_key(&name),
            Ok(_) => true,
            Err(_) => false,
        }
    }
}
