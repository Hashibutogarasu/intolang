use ast::{Expr, Path};
use native::NativeLink;
use parser::ArithmeticLexeme;
use std::collections::HashSet;
use types::{Literal, MessageTag, Type};

use super::annotation::resolve_annotation;
use super::type_name::{base_type_name, parse_type};
use crate::context::Lowerer;
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
                if let Some(source) = path.as_ident().and_then(|name| self.state.values.get(name)) {
                    let ty = resolve_annotation(source.ty.clone(), annotation)?;
                    if source.native {
                        emit::assign(&mut self.state.output, assigned_to, &ty, "p", path.first());
                        return Ok((ty.name(), ty, true));
                    }
                    return Ok((source.value.clone(), ty, false));
                }
            }
            Expr::Lambda { .. } => {
                return Err("Lambda expressions are not supported in this position".into());
            }
            Expr::Compare { .. } | Expr::Not(_) => {}
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
            let key = self.method_key(base_type_name(&receiver_type.name()), callee.last());
            if let Some(inner) = env.user_functions.get(&key) {
                let returned = self.inline_user_function(inner, Some(receiver), args, 0)?;
                return self
                    .inlined_value(returned, &key, annotation, assigned_to)
                    .map(Some);
            }
            let link = env
                .native_links
                .get(&key)
                .ok_or_else(|| format!("Undefined method `{key}`"))?;
            let mut combined = vec![Expr::ident(receiver)];
            combined.extend_from_slice(args);
            self.emit_link_call(MessageTag::LinkAssign, link, combined, assigned_to, true)?;
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
        let inferred = returned.as_ref().map(|(_, ty)| ty.clone());
        let ty = match inferred {
            Some(inferred) => resolve_annotation(inferred, annotation)?,
            None => {
                annotation.ok_or_else(|| format!("Function `{key}` does not return a value"))?
            }
        };
        if let Some((value, _)) = returned {
            emit::assign(&mut self.state.output, assigned_to, &ty, "p", &value);
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
        const REQUIRES_NUMERIC: &str = "Arithmetic requires a typed numeric value";
        let name = left.as_ident().ok_or_else(|| REQUIRES_NUMERIC.to_owned())?;
        let left_var = self
            .state
            .values
            .get(name)
            .filter(|value| value.native && value.ty.is_numeric())
            .ok_or_else(|| REQUIRES_NUMERIC.to_owned())?;
        let ty = match annotation {
            Some(annotation) if annotation != left_var.ty => {
                return Err(format!(
                    "Arithmetic type `{}` does not match operand type `{}`",
                    annotation.name(),
                    left_var.ty.name()
                ));
            }
            Some(annotation) => annotation,
            None => left_var.ty.clone(),
        };
        if !ty.is_numeric() {
            return Err("Arithmetic result type must be numeric".to_owned());
        }
        let operand = self
            .numeric_operand(right)
            .ok_or_else(|| format!("Unsupported arithmetic operand `{right}`"))?;
        let operation = ArithmeticLexeme::from(op);
        emit::arith(
            &mut self.state.output,
            assigned_to,
            name,
            operation.message_name(),
            &operand,
            &ty,
        );
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
