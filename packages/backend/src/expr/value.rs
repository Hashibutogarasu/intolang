use ast::{Expr, Path};
use errors::{Errors, Result};
use native::{CastAssign, NativeLink, Operand};
use std::collections::HashSet;
use types::{Literal, MessageTag, Type};

use super::annotation::resolve_annotation;
use super::events::{RuntimeCall, event_value, listener_value};
use super::numeric::Kind;
use super::ownership::{BORROW, DESTROY};
use super::text::Text;
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
    ) -> Result<Bound> {
        match rhs {
            Expr::Call { callee, args } => {
                let bound =
                    self.call_value(callee, args, annotation.clone(), type_params, assigned_to)?;
                if let Some(bound) = bound {
                    return Ok(bound);
                }
            }
            Expr::Binary {
                op: types::ArithmeticOperator::Add,
                ..
            } if self.is_text(rhs) => {
                let ty = resolve_annotation(Type::String, annotation)?;
                return match self.lower_text(rhs, Some(assigned_to))? {
                    Text::Const(value) => Ok((value, ty, false)),
                    Text::Slot(slot) => {
                        if slot != assigned_to {
                            emit::assign(&mut self.state.output, assigned_to, &ty, "p", &slot);
                        }
                        Ok((ty.name(), ty, true))
                    }
                };
            }
            Expr::Binary { left, op, right } => {
                return self.arithmetic_value(left, *op, right, annotation, assigned_to);
            }
            Expr::Literal(literal) => return self.literal_value(literal, annotation, assigned_to),
            Expr::Path(path) => {
                if let Some(field) = self.static_field(path) {
                    let literal = field.default_literal().ok_or_else(|| {
                        errors::Error::from(Errors::StaticFieldNoDefault {
                            field: rhs.to_string(),
                        })
                    })?;
                    return self.literal_value(literal, annotation, assigned_to);
                }
                if let Some(event) = path
                    .as_ident()
                    .filter(|name| !self.state.values.contains_key(*name))
                    .filter(|name| self.env.events.contains_key(*name))
                {
                    if self.state.destroyed_events.contains(event) {
                        return Err(Errors::Destroyed {
                            name: event.to_string(),
                        }
                        .into());
                    }
                    let (value, ty, native) = event_value(event);
                    let ty = resolve_annotation(ty, annotation)?;
                    return Ok((value, ty, native));
                }
                if path.is_member()
                    && let Some((slot, ty)) = self.lower_field_read(path)?
                {
                    let ty = resolve_annotation(ty, annotation)?;
                    emit::assign(&mut self.state.output, assigned_to, &ty, "p", &slot);
                    return Ok((ty.name(), ty, true));
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
                        if source.ty.is_numeric() && ty.is_numeric() && source.ty != ty {
                            emit::cast_assign(
                                &mut self.state.output,
                                &CastAssign {
                                    target: assigned_to.to_owned(),
                                    source: path.first().to_owned(),
                                    from: source.ty.name(),
                                    to: ty.name(),
                                },
                            );
                            return Ok((ty.name(), ty, true));
                        }
                        emit::assign(&mut self.state.output, assigned_to, &ty, "p", path.first());
                        return Ok((ty.name(), ty, true));
                    }
                    return Ok((source.value.clone(), ty, false));
                }
            }
            Expr::Lambda { params, body } => {
                let Some(expected) = annotation.filter(Self::is_function_type) else {
                    return Err(Errors::LambdaNeedsCallbackType.into());
                };
                self.lower_lambda_value(params, body, &expected, Some(assigned_to))?;
                return Ok((expected.name(), expected, true));
            }
            Expr::Thread { body } => {
                let function = self.lower_thread_body(body)?;
                let (value, ty, native) = thread_value(&function);
                let ty = resolve_annotation(ty, annotation)?;
                return Ok((value, ty, native));
            }
            Expr::Await(_) => {
                return Err(Errors::AwaitOnlyStatement.into());
            }
            Expr::Object(entries) => {
                let object = self.lower_object(entries, assigned_to)?;
                let ty = match annotation {
                    Some(annotation) => resolve_annotation(object, Some(annotation))?,
                    None => object,
                };
                return Ok((ty.name(), ty, true));
            }
            Expr::Array(items) => {
                let list = self.lower_array(items, annotation.as_ref(), assigned_to)?;
                let ty = resolve_annotation(list, annotation)?;
                return Ok((ty.name(), ty, true));
            }
            Expr::Cast { value, ty } => {
                let target = parse_type(ty.as_str(), type_params)?;
                if self.is_class_type(&target) {
                    let Some(class) = target.class_name() else {
                        unreachable!("a class type has a class name")
                    };
                    self.lower_class_cast(value, class, assigned_to)?;
                    let class = resolve_annotation(target.clone(), annotation)?;
                    return Ok((class.name(), class, true));
                }
                let value = self.lower_numeric(rhs)?;
                let Kind::Typed(ty) = value.kind else {
                    return Err(Errors::NotTypedValue {
                        value: rhs.to_string(),
                    }
                    .into());
                };
                let Operand::Slot(slot) = value.operand else {
                    return Err(Errors::NotTypedValue {
                        value: rhs.to_string(),
                    }
                    .into());
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
    ) -> Result<Option<Bound>> {
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
                RuntimeCall::Done => {
                    return Err(Errors::NoReturnValue {
                        callee: callee.to_string(),
                    }
                    .into());
                }
            }
            if let Some((class, method)) = self.static_receiver(callee) {
                let key = self.method_key(&class, &method);
                let key = self.pick_overload(&key, args)?;
                if let Some(inner) = env.user_functions.get(&key) {
                    let returned = self.inline_user_function(inner, None, args, 0)?;
                    return self
                        .inlined_value(returned, &key, annotation, assigned_to)
                        .map(Some);
                }
                let link = env.native_links.get(&key).ok_or_else(|| {
                    errors::Error::from(Errors::UndefinedStaticMethod { key: key.clone() })
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
            let (receiver, receiver_type) = self.resolve_receiver(callee)?;
            let class = base_type_name(&receiver_type.name()).to_owned();
            let method = callee.last();
            if let Some(result) =
                self.lower_list_method(&receiver, &receiver_type, method, args, assigned_to)?
            {
                let ty = result.ok_or_else(|| {
                    errors::Error::from(Errors::NoReturnValue {
                        callee: callee.to_string(),
                    })
                })?;
                let ty = resolve_annotation(ty, annotation)?;
                return Ok(Some((ty.name(), ty, true)));
            }
            if let Some(result) =
                self.lower_pointer_method(&receiver, &receiver_type, method, args, assigned_to)?
            {
                let ty = result.ok_or_else(|| {
                    errors::Error::from(Errors::NoReturnValue {
                        callee: callee.to_string(),
                    })
                })?;
                let ty = resolve_annotation(ty, annotation)?;
                return Ok(Some((ty.name(), ty, true)));
            }
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
            let key = self.pick_overload(&key, args)?;
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
                .ok_or_else(|| errors::Error::from(Errors::UndefinedMethod { key: key.clone() }))?;
            let mut combined = vec![Expr::ident(receiver.clone())];
            combined.extend_from_slice(args);
            self.emit_link_call(MessageTag::LinkAssign, link, combined, assigned_to, true)?;
            self.apply_receiver_call(&receiver, method);
            return linked_value(link, type_params, annotation).map(Some);
        }
        let function = callee.first();
        if let Some(call) = self.callable_path(function) {
            return self.call_value(&call, args, annotation, type_params, assigned_to);
        }
        let class = base_type_name(function);
        if env.class_fields.contains_key(class) {
            if env.classes.get(class).is_some_and(|info| info.is_abstract) {
                return Err(Errors::AbstractClassInstantiated {
                    class: class.to_string(),
                }
                .into());
            }
            let constructor = self.pick_overload(&format!("{class}.new"), args)?;
            if let Some(link) = env.native_links.get(&constructor) {
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
        let key = self.pick_overload(function, args)?;
        if let Some(inner) = env.user_functions.get(&key) {
            let returned = self.inline_user_function(inner, None, args, 0)?;
            return self
                .inlined_value(returned, &key, annotation, assigned_to)
                .map(Some);
        }
        if env.imported_functions.contains(function) {
            let link = env.native_links.get(&key).ok_or_else(|| {
                errors::Error::from(Errors::ImportedFunctionNoLink {
                    function: function.to_string(),
                })
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
    ) -> Result<Bound> {
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
            None => annotation.ok_or_else(|| {
                errors::Error::from(Errors::FunctionNoReturnValue {
                    key: key.to_string(),
                })
            })?,
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
    ) -> Result<Bound> {
        let ty = self.lower_binary_into(left, op, right, annotation.as_ref(), assigned_to)?;
        Ok((ty.name(), ty, true))
    }

    fn literal_value(
        &mut self,
        literal: &Literal,
        annotation: Option<Type>,
        assigned_to: &str,
    ) -> Result<Bound> {
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
) -> Result<Bound> {
    let inferred = parse_type(&link.return_type, type_params).unwrap_or(Type::Void);
    let ty = resolve_annotation(inferred, annotation)?;
    Ok((ty.name(), ty, true))
}
