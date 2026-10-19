use ast::{Expr, Path};
use native::NativeLink;
use std::collections::HashSet;
use types::{MessageTag, Type};

use super::callbacks::names_callback;
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
            self.env.user_functions.contains_key(key)
                || self.env.native_links.contains_key(key)
                || self.env.overloads.contains_key(key)
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

    /// Whether a value of type `actual` may be given where `expected` is
    /// wanted: it is assignable, or it is an instance of a class that inherits
    /// from the class `expected` names.
    pub fn accepts(&self, actual: &Type, expected: &Type) -> bool {
        if actual.is_assignable_to(expected) {
            return true;
        }
        let (Some(class), Some(wanted)) = (
            actual.referent().class_name(),
            expected.referent().class_name(),
        ) else {
            return false;
        };
        ancestors(&self.env.classes, base_type_name(class)).is_ok_and(|chain| {
            chain
                .iter()
                .any(|ancestor| ancestor.name == base_type_name(wanted))
        })
    }

    /// The key of the function a call with `args` runs, when several functions
    /// share `key` and differ in the types of their parameters: the one whose
    /// parameters fit the arguments best, preferring identical types.
    pub fn pick_overload(&self, key: &str, args: &[Expr]) -> Result<String, String> {
        let env = self.env;
        let Some(candidates) = env.overloads.get(key) else {
            return Ok(key.to_owned());
        };
        let types: Vec<Option<Type>> = args.iter().map(|arg| self.type_of(arg).ok()).collect();
        let mut best: Option<(usize, &str)> = None;
        'candidates: for candidate in candidates {
            if args.len() < candidate.required || args.len() > candidate.params.len() {
                continue;
            }
            let mut score = 0;
            for ((written, actual), arg) in candidate.params.iter().zip(&types).zip(args) {
                match (written, actual) {
                    (Some(written), Some(actual)) => {
                        let Ok(expected) = parse_type(written, &env.type_params) else {
                            continue 'candidates;
                        };
                        if actual.referent() == expected.referent() {
                            score += 2;
                        } else if self.accepts(actual, &expected) {
                            score += 1;
                        } else {
                            continue 'candidates;
                        }
                    }
                    (Some(written), None) => {
                        if matches!(arg, Expr::Lambda { .. }) && names_callback(written) {
                            score += 1;
                        } else {
                            continue 'candidates;
                        }
                    }
                    (None, _) => {}
                }
            }
            if best.is_none_or(|(top, _)| score > top) {
                best = Some((score, &candidate.key));
            }
        }
        best.map(|(_, picked)| picked.to_owned()).ok_or_else(|| {
            let given: Vec<String> = types
                .iter()
                .map(|ty| ty.as_ref().map_or_else(|| "?".to_owned(), Type::name))
                .collect();
            format!(
                "No overload of `{key}` accepts ({}); the choices are {}",
                given.join(", "),
                candidates
                    .iter()
                    .map(|candidate| format!("`{}`", candidate.key))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
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
            let declares_field = env
                .class_fields
                .get(base_type_name(&current_type.name()))
                .is_some_and(|fields| {
                    fields
                        .iter()
                        .any(|field| !field.is_static && field.name == *segment)
                });
            if declares_field
                && let Some((slot, ty)) =
                    self.lower_field_read(&Path::from(vec![current.clone(), segment.clone()]))?
            {
                current = slot;
                current_type = ty;
                continue;
            }
            let key = self.method_key(base_type_name(&current_type.name()), segment);
            if !env.native_links.contains_key(&key)
                && self.is_dynamic(base_type_name(&current_type.name()))
            {
                let (slot, ty) = self.read_dynamic_member(&current, segment)?;
                current = slot;
                current_type = ty;
                continue;
            }
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
        let key = self.pick_overload(&key, args)?;
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
