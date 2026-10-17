use std::collections::{HashMap, HashSet};

use crate::syntax::context::{ClassFields, UserFunction, Variable};
use crate::syntax::support::call::{
    inline_user_function, resolve_receiver, static_field, static_receiver,
};
use crate::syntax::support::emit::{arithmetic_operand, emit_assign, emit_link_call};
use crate::syntax::support::literal::{Literal, parse_literal};
use crate::syntax::support::text::{base_type_name, parse_arithmetic, parse_call};
use crate::types::Type;
use moduler::NativeLink;

#[allow(clippy::too_many_arguments)]
pub fn declaration_value(
    rhs: &str,
    annotation: Option<Type>,
    values: &mut HashMap<String, Variable>,
    imported_functions: &HashSet<String>,
    native_links: &HashMap<String, NativeLink>,
    class_fields: &ClassFields,
    enum_values: &HashMap<String, HashMap<String, i32>>,
    type_params: &HashSet<String>,
    output: &mut String,
    assigned_to: &str,
    temp_counter: &mut usize,
    user_functions: &HashMap<String, UserFunction>,
) -> std::result::Result<(String, Type, bool), String> {
    if let Some((function, args)) = parse_call(rhs) {
        if function.contains('.') {
            if let Some((class, method)) = static_receiver(function, values, class_fields) {
                let key = format!("{class}.{method}");
                if let Some(inner) = user_functions.get(&key) {
                    let inner = inner.clone();
                    let returned = inline_user_function(
                        output,
                        &inner,
                        None,
                        args,
                        values,
                        native_links,
                        class_fields,
                        imported_functions,
                        enum_values,
                        temp_counter,
                        user_functions,
                        0,
                    )?;
                    let inferred = returned.as_ref().map(|(_, ty)| ty.clone());
                    let ty = match inferred {
                        Some(inferred) => resolve_annotation(inferred, annotation)?,
                        None => annotation
                            .ok_or_else(|| format!("Function `{key}` does not return a value"))?,
                    };
                    if let Some((value, _)) = returned {
                        emit_assign(output, assigned_to, &ty, "p", &value);
                    }
                    return Ok((ty.name(), ty, true));
                }
                let link = native_links
                    .get(&key)
                    .ok_or_else(|| format!("Undefined static method `{key}`"))?;
                emit_link_call(output, link, args, enum_values, values, assigned_to, false)?;
                let inferred = Type::parse(&link.return_type, type_params).unwrap_or(Type::Void);
                let ty = resolve_annotation(inferred, annotation)?;
                return Ok((ty.name(), ty, true));
            }
            let (receiver, receiver_type) = resolve_receiver(
                output,
                function,
                values,
                native_links,
                enum_values,
                temp_counter,
            )?;
            let method = function
                .split('.')
                .filter(|segment| !segment.is_empty())
                .next_back()
                .unwrap_or_default();
            let key = format!("{}.{}", base_type_name(&receiver_type.name()), method);
            if let Some(inner) = user_functions.get(&key) {
                let inner = inner.clone();
                let returned = inline_user_function(
                    output,
                    &inner,
                    Some(receiver),
                    args,
                    values,
                    native_links,
                    class_fields,
                    imported_functions,
                    enum_values,
                    temp_counter,
                    user_functions,
                    0,
                )?;
                let inferred = returned.as_ref().map(|(_, ty)| ty.clone());
                let ty = match inferred {
                    Some(inferred) => resolve_annotation(inferred, annotation)?,
                    None => annotation
                        .ok_or_else(|| format!("Function `{key}` does not return a value"))?,
                };
                if let Some((value, _)) = returned {
                    emit_assign(output, assigned_to, &ty, "p", &value);
                }
                return Ok((ty.name(), ty, true));
            }
            let link = native_links
                .get(&key)
                .ok_or_else(|| format!("Undefined method `{key}`"))?;
            let combined = if args.trim().is_empty() {
                receiver
            } else {
                format!("{receiver},{args}")
            };
            emit_link_call(
                output,
                link,
                &combined,
                enum_values,
                values,
                assigned_to,
                true,
            )?;
            let inferred = Type::parse(&link.return_type, type_params).unwrap_or(Type::Void);
            let ty = resolve_annotation(inferred, annotation)?;
            return Ok((ty.name(), ty, true));
        }
        let class = base_type_name(function);
        if class_fields.contains_key(class) {
            if let Some(link) = native_links.get(&format!("{class}.new")) {
                emit_link_call(output, link, args, enum_values, values, assigned_to, false)?;
                let inferred = Type::parse(&link.return_type, type_params).unwrap_or(Type::Void);
                let ty = resolve_annotation(inferred, annotation)?;
                return Ok((ty.name(), ty, true));
            }
            let ty = resolve_annotation(
                Type::parse(function, type_params)
                    .unwrap_or_else(|_| Type::Named(function.to_owned())),
                annotation,
            )?;
            output.push('\u{1e}');
            output.push_str("NATIVE_CONSTRUCT\t");
            output.push_str(assigned_to);
            output.push('\t');
            output.push_str(class);
            output.push('\n');
            return Ok((ty.name(), ty, true));
        }
        if imported_functions.contains(function) {
            let link = native_links.get(function).ok_or_else(|| {
                format!("Imported function `{function}` has no #[link] attribute")
            })?;
            emit_link_call(output, link, args, enum_values, values, assigned_to, false)?;
            let inferred = Type::parse(&link.return_type, type_params).unwrap_or(Type::Void);
            let ty = resolve_annotation(inferred, annotation)?;
            return Ok((ty.name(), ty, true));
        }
        return Ok((
            rhs.trim_matches('"').to_owned(),
            annotation.unwrap_or(Type::String),
            false,
        ));
    }
    if let Some((left, operator, right)) = parse_arithmetic(rhs) {
        let left_var = values
            .get(left)
            .filter(|value| value.native && value.ty.is_numeric())
            .ok_or_else(|| "Arithmetic requires a typed numeric value".to_owned())?;
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
        let operand = arithmetic_operand(right, values)
            .ok_or_else(|| format!("Unsupported arithmetic operand `{right}`"))?;
        let operation = match operator {
            '+' => "add",
            '-' => "sub",
            '*' => "mul",
            '/' => "div",
            _ => unreachable!(),
        };
        output.push('\u{1e}');
        output.push_str("NATIVE_ARITH\t");
        output.push_str(assigned_to);
        output.push('\t');
        output.push_str(left);
        output.push('\t');
        output.push_str(operation);
        output.push('\t');
        output.push_str(&operand.0);
        output.push(':');
        output.push_str(&operand.1);
        output.push('\t');
        output.push_str(&ty.name());
        output.push('\n');
        return Ok((ty.name(), ty, true));
    }
    if let Some(literal) = parse_literal(rhs) {
        return match literal {
            Literal::String(value) => {
                let ty = resolve_annotation(Type::String, annotation)?;
                Ok((value, ty, false))
            }
            Literal::Int(number) => {
                let ty = resolve_annotation(Type::Int32, annotation)?;
                emit_assign(output, assigned_to, &ty, "n", &number.to_string());
                Ok((ty.name(), ty, true))
            }
            Literal::Float(number) => {
                let ty = resolve_annotation(Type::Float64, annotation)?;
                emit_assign(output, assigned_to, &ty, "n", &number.to_string());
                Ok((ty.name(), ty, true))
            }
            Literal::Bool(flag) => {
                let ty = resolve_annotation(Type::Boolean, annotation)?;
                emit_assign(output, assigned_to, &ty, "n", if flag { "1" } else { "0" });
                Ok((ty.name(), ty, true))
            }
        };
    }
    if let Some(field) = static_field(rhs, class_fields) {
        let default = field.default.clone().unwrap_or_default();
        let literal = parse_literal(&default)
            .ok_or_else(|| format!("Static field `{rhs}` has no constant default"))?;
        return match literal {
            Literal::String(value) => {
                let ty = resolve_annotation(Type::String, annotation)?;
                Ok((value, ty, false))
            }
            Literal::Int(number) => {
                let ty = resolve_annotation(Type::Int32, annotation)?;
                emit_assign(output, assigned_to, &ty, "n", &number.to_string());
                Ok((ty.name(), ty, true))
            }
            Literal::Float(number) => {
                let ty = resolve_annotation(Type::Float64, annotation)?;
                emit_assign(output, assigned_to, &ty, "n", &number.to_string());
                Ok((ty.name(), ty, true))
            }
            Literal::Bool(flag) => {
                let ty = resolve_annotation(Type::Boolean, annotation)?;
                emit_assign(output, assigned_to, &ty, "n", if flag { "1" } else { "0" });
                Ok((ty.name(), ty, true))
            }
        };
    }
    if let Some(source) = values.get(rhs) {
        let ty = resolve_annotation(source.ty.clone(), annotation)?;
        if source.native {
            emit_assign(output, assigned_to, &ty, "p", rhs.trim());
            return Ok((ty.name(), ty, true));
        }
        return Ok((source.value.clone(), ty, false));
    }
    Ok((
        rhs.trim_matches('"').to_owned(),
        annotation.unwrap_or(Type::String),
        false,
    ))
}

pub fn resolve_annotation(
    inferred: Type,
    annotation: Option<Type>,
) -> std::result::Result<Type, String> {
    match annotation {
        Some(target) => {
            if inferred.is_assignable_to(&target) {
                Ok(target)
            } else {
                Err(format!(
                    "Type mismatch: expected `{}` but found `{}`",
                    target.name(),
                    inferred.name()
                ))
            }
        }
        None => Ok(inferred),
    }
}
