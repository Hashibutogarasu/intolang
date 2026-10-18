use ast::{Expr, Path};
use native::NativeLink;
use std::collections::HashSet;
use types::{MessageTag, Type};

use super::type_name::{base_type_name, parse_type};
use crate::collect::ancestors;
use crate::context::{FieldDef, Lowerer, Variable};

impl<'a> Lowerer<'a> {
    /// Resolve a static receiver: `Class.method` where `Class` names a declared
    /// class rather than a bound value. Returns the class and method names.
    pub fn static_receiver(&self, path: &Path) -> Option<(String, String)> {
        let [class, method] = path.segments() else {
            return None;
        };
        if self.state.values.contains_key(class) {
            return None;
        }
        self.env
            .class_fields
            .contains_key(class)
            .then(|| (class.clone(), method.clone()))
    }

    /// The `Class.method` key a call resolves to: the class itself, then its
    /// ancestors from the nearest, so a subclass finds inherited members and
    /// its own `override` wins.
    pub fn method_key(&self, class: &str, method: &str) -> String {
        let defined = |key: &str| {
            self.env.user_functions.contains_key(key) || self.env.native_links.contains_key(key)
        };
        let own = format!("{class}.{method}");
        if defined(&own) {
            return own;
        }
        if let Ok(chain) = ancestors(&self.env.classes, class) {
            for ancestor in chain {
                let key = format!("{}.{method}", ancestor.name);
                if defined(&key) {
                    return key;
                }
            }
        }
        own
    }

    /// Resolve a static field constant reference `Class.field`.
    pub fn static_field(&self, path: &Path) -> Option<&'a FieldDef> {
        let [class, field] = path.segments() else {
            return None;
        };
        self.env
            .class_fields
            .get(class)?
            .iter()
            .find(|def| def.is_static && def.name == *field)
    }

    /// Walk the member chain of a method call path, emitting a call for each
    /// intermediate member into a temporary. Returns the name holding the final
    /// receiver and its type.
    pub fn resolve_receiver(&mut self, path: &Path) -> Result<(String, Type), String> {
        let segments = path.segments();
        if segments.len() < 2 {
            return Err(format!("`{path}` is not a method call"));
        }
        let env = self.env;
        let receiver = &segments[0];
        let mut current = receiver.clone();
        let mut current_type = self
            .state
            .values
            .get(receiver)
            .map(|variable| variable.ty.clone())
            .ok_or_else(|| format!("Unknown receiver `{receiver}`"))?;
        for segment in &segments[1..segments.len() - 1] {
            let key = self.method_key(base_type_name(&current_type.name()), segment);
            let link = env.native_links.get(&key).ok_or_else(|| {
                format!("Undefined member `{segment}` on `{}`", current_type.name())
            })?;
            let temporary = format!("__member{}", self.state.temp_counter);
            self.state.temp_counter += 1;
            self.emit_link_call(
                MessageTag::LinkAssign,
                link,
                vec![Expr::ident(current.clone())],
                &temporary,
                true,
            )?;
            let ty = parse_type(&link.return_type, &HashSet::new()).unwrap_or(Type::Pointer);
            self.state
                .values
                .insert(temporary.clone(), Variable::temporary(ty.clone()));
            current = temporary;
            current_type = ty;
        }
        Ok((current, current_type))
    }

    /// Resolve the native link of a method call path and the argument list
    /// with the receiver in front.
    pub fn resolve_method(
        &mut self,
        path: &Path,
        args: &[Expr],
    ) -> Result<(&'a NativeLink, Vec<Expr>), String> {
        let (receiver, receiver_type) = self.resolve_receiver(path)?;
        let key = self.method_key(base_type_name(&receiver_type.name()), path.last());
        let link = self
            .env
            .native_links
            .get(&key)
            .ok_or_else(|| format!("Undefined method `{key}`"))?;
        let mut combined = vec![Expr::ident(receiver)];
        combined.extend_from_slice(args);
        Ok((link, combined))
    }
}
