use std::collections::{HashMap, HashSet};

use crate::imports;
use crate::syntax::context::{UserFunction, Variable};
use crate::syntax::support::emit::{comparison_operand, emit_comparison, emit_link_call};
use crate::syntax::support::text::{
    apply_substitutions, boolean_literal, parse_call, parse_comparison, split_argument_list,
};
use crate::types::Type;

#[allow(clippy::too_many_arguments)]
pub fn resolve_method(
    output: &mut String,
    path: &str,
    args: &str,
    values: &mut HashMap<String, Variable>,
    native_links: &HashMap<String, imports::NativeLink>,
    enum_values: &HashMap<String, HashMap<String, i32>>,
    temp_counter: &mut usize,
) -> std::result::Result<(imports::NativeLink, String), String> {
    let segments = path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.len() < 2 {
        return Err(format!("`{path}` is not a method call"));
    }
    let receiver = segments[0];
    let mut current = receiver.to_owned();
    let mut current_type = values
        .get(receiver)
        .map(|variable| variable.ty.clone())
        .ok_or_else(|| format!("Unknown receiver `{receiver}`"))?;
    for segment in &segments[1..segments.len() - 1] {
        let key = format!("{}.{}", current_type.name(), segment);
        let link = native_links
            .get(&key)
            .ok_or_else(|| format!("Undefined member `{segment}` on `{}`", current_type.name()))?;
        let temporary = format!("__member{}", *temp_counter);
        *temp_counter += 1;
        emit_link_call(
            output,
            link,
            &current,
            enum_values,
            &*values,
            &temporary,
            true,
        )?;
        let ty = Type::parse(&link.return_type, &HashSet::new()).unwrap_or(Type::Pointer);
        values.insert(
            temporary.clone(),
            Variable {
                value: ty.name(),
                ty: ty.clone(),
                memory_backed: false,
                allocated: true,
                lazy: false,
                unloaded: false,
                declared_in_init: false,
                native: true,
            },
        );
        current = temporary;
        current_type = ty;
    }
    let method = segments[segments.len() - 1];
    let key = format!("{}.{}", current_type.name(), method);
    let link = native_links
        .get(&key)
        .ok_or_else(|| format!("Undefined method `{key}`"))?
        .clone();
    let combined = if args.trim().is_empty() {
        current
    } else {
        format!("{current},{args}")
    };
    Ok((link, combined))
}

#[allow(clippy::too_many_arguments)]
pub fn resolve_receiver(
    output: &mut String,
    path: &str,
    values: &mut HashMap<String, Variable>,
    native_links: &HashMap<String, imports::NativeLink>,
    enum_values: &HashMap<String, HashMap<String, i32>>,
    temp_counter: &mut usize,
) -> std::result::Result<(String, Type), String> {
    let segments = path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.len() < 2 {
        return Err(format!("`{path}` is not a method call"));
    }
    let receiver = segments[0];
    let mut current = receiver.to_owned();
    let mut current_type = values
        .get(receiver)
        .map(|variable| variable.ty.clone())
        .ok_or_else(|| format!("Unknown receiver `{receiver}`"))?;
    for segment in &segments[1..segments.len() - 1] {
        let key = format!("{}.{}", current_type.name(), segment);
        let link = native_links
            .get(&key)
            .ok_or_else(|| format!("Undefined member `{segment}` on `{}`", current_type.name()))?;
        let temporary = format!("__member{}", *temp_counter);
        *temp_counter += 1;
        emit_link_call(
            output,
            link,
            &current,
            enum_values,
            &*values,
            &temporary,
            true,
        )?;
        let ty = Type::parse(&link.return_type, &HashSet::new()).unwrap_or(Type::Pointer);
        values.insert(
            temporary.clone(),
            Variable {
                value: ty.name(),
                ty: ty.clone(),
                memory_backed: false,
                allocated: true,
                lazy: false,
                unloaded: false,
                declared_in_init: false,
                native: true,
            },
        );
        current = temporary;
        current_type = ty;
    }
    Ok((current, current_type))
}

#[allow(clippy::too_many_arguments)]
pub fn inline_user_function(
    output: &mut String,
    function: &UserFunction,
    receiver: Option<String>,
    args: &str,
    values: &mut HashMap<String, Variable>,
    native_links: &HashMap<String, imports::NativeLink>,
    enum_values: &HashMap<String, HashMap<String, i32>>,
    temp_counter: &mut usize,
    user_functions: &HashMap<String, UserFunction>,
    depth: usize,
) -> std::result::Result<Option<(String, Type)>, String> {
    if depth > 32 {
        return Err(format!(
            "Recursive call to `{}` is not supported",
            function.name
        ));
    }
    let mut arguments = split_argument_list(args);
    if arguments.len() > function.params.len() {
        return Err(format!(
            "Function `{}` expects {} argument(s) but received {}",
            function.name,
            function.params.len(),
            arguments.len()
        ));
    }
    for index in arguments.len()..function.params.len() {
        let param = &function.params[index];
        match &param.default {
            Some(default) => arguments.push(default.clone()),
            None => {
                return Err(format!("Missing required argument `{}`", param.name));
            }
        }
    }
    let mut substitutions: Vec<(String, String)> = Vec::new();
    match (function.receiver.as_ref(), receiver) {
        (Some(_), Some(receiver)) => substitutions.push(("this".to_owned(), receiver)),
        (Some(_), None) => {
            return Err(format!("Method `{}` requires a receiver", function.name));
        }
        (None, Some(_)) => {
            return Err(format!("Function `{}` is not a method", function.name));
        }
        (None, None) => {}
    }
    for (param, argument) in function.params.iter().zip(arguments.iter()) {
        substitutions.push((param.name.clone(), argument.trim().to_owned()));
    }
    let mut blocks: Vec<char> = Vec::new();
    let mut returned: Option<(String, Type)> = None;
    for body_line in &function.body {
        let substituted = apply_substitutions(body_line, &substitutions);
        lower_inline_statement(
            output,
            &substituted,
            values,
            native_links,
            enum_values,
            temp_counter,
            user_functions,
            depth,
            &mut blocks,
            &mut returned,
        )?;
    }
    if !blocks.is_empty() {
        return Err(format!("Unbalanced braces in `{}`", function.name));
    }
    Ok(returned)
}

#[allow(clippy::too_many_arguments)]
pub fn lower_inline_statement(
    output: &mut String,
    line: &str,
    values: &mut HashMap<String, Variable>,
    native_links: &HashMap<String, imports::NativeLink>,
    enum_values: &HashMap<String, HashMap<String, i32>>,
    temp_counter: &mut usize,
    user_functions: &HashMap<String, UserFunction>,
    depth: usize,
    blocks: &mut Vec<char>,
    returned: &mut Option<(String, Type)>,
) -> std::result::Result<(), String> {
    let line = line.trim().trim_end_matches(';').trim();
    if line.is_empty() || line == "break" {
        return Ok(());
    }
    if line == "return" {
        return Ok(());
    }
    if let Some(expression) = line.strip_prefix("return ") {
        let expression = expression.trim();
        let variable = values
            .get(expression)
            .ok_or_else(|| format!("Unsupported return value `{expression}`"))?;
        *returned = Some((expression.to_owned(), variable.ty.clone()));
        return Ok(());
    }
    if line == "}" {
        match blocks.pop() {
            Some('f') => {
                output.push('\u{1e}');
                output.push_str("NATIVE_IF_END\n");
            }
            _ => return Err("Unexpected closing brace in function body".to_owned()),
        }
        return Ok(());
    }
    if line == "} else {" {
        if blocks.last() != Some(&'f') {
            return Err("`else` must follow an `if` block".to_owned());
        }
        output.push('\u{1e}');
        output.push_str("NATIVE_IF_ELSE\n");
        return Ok(());
    }
    if let Some(condition) = line
        .strip_prefix("if ")
        .and_then(|value| value.strip_suffix('{'))
        .map(str::trim)
    {
        let condition = condition
            .strip_prefix('(')
            .and_then(|value| value.strip_suffix(')'))
            .unwrap_or(condition)
            .trim();
        if let Some((left, operator, right)) = parse_comparison(condition) {
            let left_var = values
                .get(left)
                .filter(|value| value.native && value.ty.is_numeric())
                .ok_or_else(|| "if condition requires an integer or float value".to_owned())?;
            let operand = comparison_operand(right, values)
                .ok_or_else(|| "if condition requires a numeric literal or value".to_owned())?;
            let ty = left_var.ty.clone();
            emit_comparison(output, "NATIVE_IF_BEGIN", left, operator, &operand, &ty);
        } else if let Some(flag) = boolean_literal(condition) {
            output.push('\u{1e}');
            output.push_str("NATIVE_IF_CONST\t");
            output.push_str(if flag { "1" } else { "0" });
            output.push('\n');
        } else {
            return Err(format!(
                "Unsupported if condition in function body: `{condition}`"
            ));
        }
        blocks.push('f');
        return Ok(());
    }
    let Some((call, call_args)) = parse_call(line) else {
        return Err(format!("Unsupported statement in function body: `{line}`"));
    };
    if call.contains('.') {
        let (receiver, receiver_type) = resolve_receiver(
            output,
            call,
            values,
            native_links,
            enum_values,
            temp_counter,
        )?;
        let method = call
            .split('.')
            .filter(|segment| !segment.is_empty())
            .next_back()
            .unwrap_or_default();
        let key = format!("{}.{}", receiver_type.name(), method);
        if let Some(inner) = user_functions.get(&key) {
            let inner = inner.clone();
            return inline_user_function(
                output,
                &inner,
                Some(receiver),
                call_args,
                values,
                native_links,
                enum_values,
                temp_counter,
                user_functions,
                depth + 1,
            )
            .map(|_| ());
        }
        let link = native_links
            .get(&key)
            .ok_or_else(|| format!("Undefined method `{key}`"))?;
        let combined = if call_args.trim().is_empty() {
            receiver
        } else {
            format!("{receiver},{call_args}")
        };
        return emit_link_call(output, link, &combined, enum_values, values, "", true);
    }
    if let Some(inner) = user_functions.get(call) {
        let inner = inner.clone();
        return inline_user_function(
            output,
            &inner,
            None,
            call_args,
            values,
            native_links,
            enum_values,
            temp_counter,
            user_functions,
            depth + 1,
        )
        .map(|_| ());
    }
    let link = native_links
        .get(call)
        .ok_or_else(|| format!("Undefined function `{call}`"))?;
    emit_link_call(output, link, call_args, enum_values, values, "", false)
}
