use ast::{Expr, Path};
use native::{NativeLink, Operand};
use std::collections::HashSet;
use types::{Literal, MessageTag, Type};

use super::annotation::resolve_annotation;
use super::events::{RuntimeCall, event_value, listener_value};
use super::numeric::Kind;
use super::ownership::{BORROW, DESTROY};
use super::threads::{task_value, thread_value};
use super::type_name::{base_type_name, parse_type};
use crate::context::{Lowerer, Tracking, UserFunction};
use crate::emit;

/// The value a declaration binds: the text kept for it, its type, and whether
/// it lives in a native slot.
type Bound = (String, Type, bool);

impl<'a> Lowerer<'a> {
    /// Lower the right-hand side of a binding named `assigned_to`, emitting
    /// whatever records produce the value.
    pub fn declaration_value(
        &mut self,
        rhs: &Expr,
        annotation: Option<Type>,
        type_params: &HashSet<String>,
        assigned_to: &str,
    ) -> Result<Bound, String> {
        match rhs {
            Expr::Call { callee, args } => {
                let bound =
                    self.call_value(callee, args, annotation.clone(), type_params, assigned_to)?;
                if let Some(bound) = bound {
                    return Ok(bound);
                }
            }
            Expr::Binary { left, op, right } => {
                return self.arithmetic_value(left, *op, right, annotation, assigned_to);
            }
            Expr::Literal(literal) => return self.literal_value(literal, annotation, assigned_to),
            Expr::Path(path) => {
                if let Some(field) = self.static_field(path) {
                    let literal = field
                        .default_literal()
                        .ok_or_else(|| format!("Static field `{rhs}` has no constant default"))?;
                    return self.literal_value(literal, annotation, assigned_to);
                }
                if let Some(event) = path
                    .as_ident()
                    .filter(|name| !self.state.values.contains_key(*name))
                    .filter(|name| self.env.events.contains_key(*name))
                {
                    if self.state.destroyed_events.contains(event) {
                        return Err(format!("`{event}` was destroyed"));
                    }
                    let (value, ty, native) = event_value(event);
                    let ty = resolve_annotation(ty, annotation)?;
                    return Ok((value, ty, native));
                }
                if let Some(name) = path.as_ident() {
                    self.ensure_usable(name)?;
                    if matches!(annotation, Some(Type::Reference(_)))
                        && let Some(source) = self.state.values.get(name)
                        && source.native
                        && self.is_ownership_method(base_type_name(&source.ty.name()), DESTROY)
                    {
                        let source_type = source.ty.clone();
                        let ty =
                            self.lower_ownership_value(name, &source_type, BORROW, assigned_to)?;
                        let ty = resolve_annotation(ty, annotation)?;
                        return Ok((ty.name(), ty, true));
                    }
                }
                if let Some(source) = path.as_ident().and_then(|name| self.state.values.get(name)) {
                    let ty = resolve_annotation(source.ty.clone(), annotation)?;
                    if source.native {
                        let shares_instance =
                            self.is_ownership_method(base_type_name(&source.ty.name()), DESTROY);
                        if shares_instance {
                            self.state.pending_tracking = Some(Tracking {
                                reference: true,
                                ..Tracking::default()
                            });
                        }
                        emit::assign(&mut self.state.output, assigned_to, &ty, "p", path.first());
                        return Ok((ty.name(), ty, true));
                    }
                    return Ok((source.value.clone(), ty, false));
                }
            }
            Expr::Lambda { .. } => {
                return Err("Lambda expressions are not supported in this position".into());
            }
            Expr::Thread { body } => {
                let function = self.lower_thread_body(body)?;
                let (value, ty, native) = thread_value(&function);
                let ty = resolve_annotation(ty, annotation)?;
                return Ok((value, ty, native));
            }
            Expr::Await(_) => {
                return Err("`await` can only be used as a statement".into());
            }
            Expr::Cast { value, ty } => {
                let target =
                    parse_type(ty.as_str(), type_params).map_err(|error| error.to_string())?;
                if self.is_class_type(&target) {
                    let Type::Named(class) = &target else {
                        unreachable!("a class type is a named type")
                    };
                    let class = self.lower_class_cast(value, class, assigned_to)?;
                    let class = resolve_annotation(class, annotation)?;
                    return Ok((class.name(), class, true));
                }
                let value = self.lower_numeric(rhs)?;
                let Kind::Typed(ty) = value.kind else {
                    return Err(format!("`{rhs}` is not a typed value"));
                };
                let Operand::Slot(slot) = value.operand else {
                    return Err(format!("`{rhs}` is not a typed value"));
                };
                emit::assign(&mut self.state.output, assigned_to, &ty, "p", &slot);
                let ty = resolve_annotation(ty, annotation)?;
                return Ok((ty.name(), ty, true));
            }
            Expr::Compare { .. } | Expr::Logical { .. } | Expr::Not(_) => {
                let slot = self.lower_boolean(rhs)?;
                emit::assign(
                    &mut self.state.output,
                    assigned_to,
                    &Type::Boolean,
                    "p",
                    &slot,
                );
                let ty = resolve_annotation(Type::Boolean, annotation)?;
                return Ok((ty.name(), ty, true));
            }
        }
        Ok((
            rhs.to_string().trim_matches('"').to_owned(),
            annotation.unwrap_or(Type::String),
            false,
        ))
    }

    fn call_value(
        &mut self,
        callee: &Path,
        args: &[Expr],
        annotation: Option<Type>,
        type_params: &HashSet<String>,
        assigned_to: &str,
    ) -> Result<Option<Bound>, String> {
        let env = self.env;
        if callee.is_member() {
            match self.lower_event_call(callee, args, assigned_to)? {
                RuntimeCall::NotRuntime => {}
                RuntimeCall::Listener(event) => {
                    let (value, ty, native) = listener_value(&event);
                    let ty = resolve_annotation(ty, annotation)?;
                    return Ok(Some((value, ty, native)));
                }
                RuntimeCall::Task => {
                    let (value, ty, native) = task_value();
                    let ty = resolve_annotation(ty, annotation)?;
                    return Ok(Some((value, ty, native)));
                }
                RuntimeCall::Done => return Err(format!("`{callee}` does not return a value")),
            }
            if let Some((class, method)) = self.static_receiver(callee) {
                let key = self.method_key(&class, &method);
                if let Some(inner) = env.user_functions.get(&key) {
                    let returned = self.inline_user_function(inner, None, args, 0)?;
                    return self
                        .inlined_value(returned, &key, annotation, assigned_to)
                        .map(Some);
                }
                let link = env
                    .native_links
                    .get(&key)
                    .ok_or_else(|| format!("Undefined static method `{key}`"))?;
                self.emit_link_call(
                    MessageTag::LinkAssign,
                    link,
                    args.to_vec(),
                    assigned_to,
                    false,
                )?;
                return linked_value(link, type_params, annotation).map(Some);
            }
            let (receiver, receiver_type) = self.resolve_receiver(callee)?;
            let class = base_type_name(&receiver_type.name()).to_owned();
            let method = callee.last();
            if self.is_ownership_method(&class, method)
                && matches!(method, "copy" | "move" | "borrow")
            {
                let ty =
                    self.lower_ownership_value(&receiver, &receiver_type, method, assigned_to)?;
                let ty = resolve_annotation(ty, annotation)?;
                return Ok(Some((ty.name(), ty, true)));
            }
            self.check_receiver_call(&receiver, method)?;
            let key = self.method_key(&class, method);
            if let Some(inner) = env.user_functions.get(&key) {
                let returned = self.inline_user_function(inner, Some(receiver.clone()), args, 0)?;
                self.apply_receiver_call(&receiver, method);
                return self
                    .inlined_value(returned, &key, annotation, assigned_to)
                    .map(Some);
            }
            let link = env
                .native_links
                .get(&key)
                .ok_or_else(|| format!("Undefined method `{key}`"))?;
            let mut combined = vec![Expr::ident(receiver.clone())];
            combined.extend_from_slice(args);
            self.emit_link_call(MessageTag::LinkAssign, link, combined, assigned_to, true)?;
            self.apply_receiver_call(&receiver, method);
            return linked_value(link, type_params, annotation).map(Some);
        }
        let function = callee.first();
        let class = base_type_name(function);
        if env.class_fields.contains_key(class) {
            if env.classes.get(class).is_some_and(|info| info.is_abstract) {
                return Err(format!("Cannot instantiate the abstract class `{class}`"));
            }
            if let Some(link) = env.native_links.get(&format!("{class}.new")) {
                self.emit_link_call(
                    MessageTag::LinkAssign,
                    link,
                    args.to_vec(),
                    assigned_to,
                    false,
                )?;
                return linked_value(link, type_params, annotation).map(Some);
            }
            let named = parse_type(function, type_params)
                .unwrap_or_else(|_| Type::Named(function.to_owned()));
            let ty = resolve_annotation(named, annotation)?;
            emit::construct(&mut self.state.output, assigned_to, class);
            return Ok(Some((ty.name(), ty, true)));
        }
        if let Some(inner) = env.user_functions.get(function) {
            let returned = self.inline_user_function(inner, None, args, 0)?;
            return self
                .inlined_value(returned, function, annotation, assigned_to)
                .map(Some);
        }
        if env.imported_functions.contains(function) {
            let link = env.native_links.get(function).ok_or_else(|| {
                format!("Imported function `{function}` has no #[link] attribute")
            })?;
            self.emit_link_call(
                MessageTag::LinkAssign,
                link,
                args.to_vec(),
                assigned_to,
                false,
            )?;
            return linked_value(link, type_params, annotation).map(Some);
        }
        Ok(None)
    }

    fn inlined_value(
        &mut self,
        returned: Option<(String, Type)>,
        key: &str,
        annotation: Option<Type>,
        assigned_to: &str,
    ) -> Result<Bound, String> {
        let reference = self
            .env
            .user_functions
            .get(key)
            .is_some_and(UserFunction::returns_reference);
        let inferred = returned.as_ref().map(|(_, ty)| {
            if reference {
                Type::Reference(Box::new(ty.referent().clone()))
            } else {
                ty.clone()
            }
        });
        let ty = match inferred {
            Some(inferred) => resolve_annotation(inferred, annotation)?,
            None => {
                annotation.ok_or_else(|| format!("Function `{key}` does not return a value"))?
            }
        };
        if let Some((value, _)) = returned {
            emit::assign(&mut self.state.output, assigned_to, &ty, "p", &value);
            if reference {
                self.state.pending_tracking = Some(self.borrow_of(&value));
            }
        }
        Ok((ty.name(), ty, true))
    }

    fn arithmetic_value(
        &mut self,
        left: &Expr,
        op: types::ArithmeticOperator,
        right: &Expr,
        annotation: Option<Type>,
        assigned_to: &str,
    ) -> Result<Bound, String> {
        let ty = self.lower_binary_into(left, op, right, annotation.as_ref(), assigned_to)?;
        Ok((ty.name(), ty, true))
    }

    fn literal_value(
        &mut self,
        literal: &Literal,
        annotation: Option<Type>,
        assigned_to: &str,
    ) -> Result<Bound, String> {
        let (inferred, native) = match literal {
            Literal::String(_) => (Type::String, None),
            Literal::Char(value) => (Type::Char, Some(u32::from(*value).to_string())),
            Literal::Int(number) => (Type::Int32, Some(number.to_string())),
            Literal::Float(number) => (Type::Float64, Some(number.to_string())),
            Literal::Bool(flag) => (
                Type::Boolean,
                Some(if *flag { "1" } else { "0" }.to_owned()),
            ),
        };
        let ty = resolve_annotation(inferred, annotation)?;
        match (literal, native) {
            (Literal::String(value), _) => Ok((value.clone(), ty, false)),
            (_, Some(value)) => {
                emit::assign(&mut self.state.output, assigned_to, &ty, "n", &value);
                Ok((ty.name(), ty, true))
            }
            (_, None) => unreachable!("numeric and boolean literals carry a native value"),
        }
    }
}

fn linked_value(
    link: &NativeLink,
    type_params: &HashSet<String>,
    annotation: Option<Type>,
) -> Result<Bound, String> {
    let inferred = parse_type(&link.return_type, type_params).unwrap_or(Type::Void);
    let ty = resolve_annotation(inferred, annotation)?;
    Ok((ty.name(), ty, true))
}
