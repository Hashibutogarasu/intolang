use anyhow::{Result, anyhow};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::{imports, lexer::tokenize, types::Type, validation};

/// Parse the currently supported statement subset and lower it to output.
pub(super) fn lower(source: &str, packages_dir: Option<&Path>) -> Result<String> {
    lower_named(source, packages_dir, "<source>")
}

struct Variable {
    value: String,
    ty: Type,
    memory_backed: bool,
    allocated: bool,
    lazy: bool,
    unloaded: bool,
    declared_in_init: bool,
    native: bool,
}

type ClassFields = HashMap<String, Vec<(String, String)>>;

#[derive(Clone)]
struct UserFunction {
    name: String,
    receiver: Option<String>,
    params: Vec<(String, Type)>,
    body: Vec<String>,
}

enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

fn diagnostic(path: &str, line: usize, scope: &str, message: &str) -> anyhow::Error {
    anyhow!("{message}\n\nInto language backtrace:\n  0: {scope} at {path}:{line}")
}

pub(super) fn lower_named(
    source: &str,
    packages_dir: Option<&Path>,
    source_path: &str,
) -> Result<String> {
    let _tokens = tokenize(source)?;
    validation::function_scopes(source, source_path)?;
    let imported_functions = if let Some(packages_dir) = packages_dir {
        imports::resolve(source, packages_dir)?
    } else {
        Default::default()
    };
    let native_links = if let Some(packages_dir) = packages_dir {
        imports::resolve_links(source, packages_dir)?
    } else {
        HashMap::new()
    };

    let type_params = collect_type_params(source);
    let mut values: HashMap<String, Variable> = HashMap::new();
    let mut temp_counter = 0usize;
    let (mut enum_values, enum_lines) = parse_enums(source, source_path)?;
    let module_sources = if let Some(packages_dir) = packages_dir {
        imports::imported_module_sources(source, packages_dir)?
    } else {
        Vec::new()
    };
    for module in &module_sources {
        let (module_enums, _) = parse_enums(&module, source_path)?;
        for (name, variants) in module_enums {
            enum_values.entry(name).or_insert(variants);
        }
    }
    let class_fields = module_sources
        .iter()
        .flat_map(|module| parse_classes(module))
        .collect::<HashMap<_, _>>();
    let mut user_functions: HashMap<String, UserFunction> = HashMap::new();
    for module in &module_sources {
        for function in parse_user_functions(module) {
            user_functions.insert(function.name.clone(), function);
        }
    }
    for function in parse_user_functions(source) {
        user_functions.insert(function.name.clone(), function);
    }
    let mut output = String::new();
    for (class, fields) in &class_fields {
        output.push('\u{1e}');
        output.push_str("NATIVE_CLASS\t");
        output.push_str(class);
        for (field, ty) in fields {
            output.push('\t');
            output.push_str(field);
            output.push(':');
            output.push_str(ty);
        }
        output.push('\n');
    }
    let mut in_init = false;
    let mut init_has_run = false;
    let mut blocks = Vec::new();
    let mut skip_depth = 0i32;

    for (index, raw) in source.lines().enumerate() {
        let line_number = index + 1;
        let code = strip_line_comment(raw);
        let line = code.trim();
        if line.starts_with("import ") || line.is_empty() {
            continue;
        }
        if enum_lines.contains(&line_number) {
            continue;
        }
        if skip_depth > 0 {
            skip_depth += brace_delta(line);
            continue;
        }
        if (line.starts_with("pub fun") || line.starts_with("fun ")) && line.contains('{') {
            let delta = brace_delta(line);
            if delta > 0 {
                skip_depth = delta;
            }
            continue;
        }
        if line.starts_with("init") && line.contains('{') {
            in_init = true;
            init_has_run = true;
            blocks.push('i');
            continue;
        }
        let scope_name = if in_init { "init" } else { "module" };
        if line == "} else {" {
            if blocks.last() != Some(&'f') {
                return Err(diagnostic(
                    source_path,
                    line_number,
                    scope_name,
                    "`else` must follow an `if` block",
                ));
            }
            output.push('\u{1e}');
            output.push_str("NATIVE_IF_ELSE\n");
            continue;
        }
        if line == "}" {
            match blocks.pop() {
                Some('w') => {
                    output.push('\u{1e}');
                    output.push_str("NATIVE_LOOP_END\n");
                }
                Some('f') => {
                    output.push('\u{1e}');
                    output.push_str("NATIVE_IF_END\n");
                }
                Some('i') => in_init = false,
                _ => {
                    return Err(diagnostic(
                        source_path,
                        line_number,
                        scope_name,
                        "Unexpected closing brace",
                    ));
                }
            }
            continue;
        }
        if let Some(condition) = line
            .strip_prefix("if ")
            .and_then(|v| v.strip_suffix('{'))
            .map(str::trim)
        {
            let condition = condition
                .strip_prefix('(')
                .and_then(|v| v.strip_suffix(')'))
                .unwrap_or(condition)
                .trim();
            if let Some((left, operator, right)) = parse_comparison(condition) {
                let left_var = values
                    .get(left)
                    .filter(|value| value.native && value.ty.is_numeric())
                    .ok_or_else(|| {
                        diagnostic(
                            source_path,
                            line_number,
                            scope_name,
                            "if condition requires an integer or float value",
                        )
                    })?;
                let operand = comparison_operand(right, &values).ok_or_else(|| {
                    diagnostic(
                        source_path,
                        line_number,
                        scope_name,
                        "if condition requires a numeric literal or value",
                    )
                })?;
                let ty = left_var.ty.clone();
                emit_comparison(
                    &mut output,
                    "NATIVE_IF_BEGIN",
                    left,
                    operator,
                    &operand,
                    &ty,
                );
            } else {
                let Some((function, args)) = parse_call(condition) else {
                    return Err(diagnostic(
                        source_path,
                        line_number,
                        scope_name,
                        "if condition must compare numeric values or call a Boolean function",
                    ));
                };
                let (link, combined) = if function.contains('.') {
                    resolve_method(
                        &mut output,
                        function,
                        args,
                        &mut values,
                        &native_links,
                        &enum_values,
                        &mut temp_counter,
                    )
                    .map_err(|message| {
                        diagnostic(source_path, line_number, scope_name, &message)
                    })?
                } else {
                    if !imported_functions.contains(function) {
                        return Err(diagnostic(
                            source_path,
                            line_number,
                            scope_name,
                            &format!("Function `{function}` is not imported"),
                        ));
                    }
                    let link = native_links.get(function).ok_or_else(|| {
                        diagnostic(
                            source_path,
                            line_number,
                            scope_name,
                            &format!("Function `{function}` has no native link"),
                        )
                    })?;
                    (link.clone(), args.to_owned())
                };
                if link.return_type != "Boolean" {
                    return Err(diagnostic(
                        source_path,
                        line_number,
                        scope_name,
                        &format!("if condition `{function}` must return Boolean"),
                    ));
                }
                let marker_start = output.len();
                emit_link_call(&mut output, &link, &combined, &enum_values, &values, "").map_err(
                    |message| diagnostic(source_path, line_number, scope_name, &message),
                )?;
                let marker =
                    output[marker_start..].replacen("NATIVE_LINK_ASSIGN", "NATIVE_IF_CALL", 1);
                output.replace_range(marker_start.., &marker);
            }
            blocks.push('f');
            continue;
        }
        if let Some(condition) = line
            .strip_prefix("while ")
            .and_then(|v| v.strip_suffix('{'))
            .map(str::trim)
        {
            let condition = condition
                .strip_prefix('(')
                .and_then(|v| v.strip_suffix(')'))
                .unwrap_or(condition)
                .trim();
            if let Some((left, operator, right)) = parse_comparison(condition) {
                let left_var = values
                    .get(left)
                    .filter(|value| value.native && value.ty.is_numeric())
                    .ok_or_else(|| {
                        diagnostic(
                            source_path,
                            line_number,
                            scope_name,
                            "while condition requires an integer or float value",
                        )
                    })?;
                let operand = comparison_operand(right, &values).ok_or_else(|| {
                    diagnostic(
                        source_path,
                        line_number,
                        scope_name,
                        "while condition requires a numeric literal or value",
                    )
                })?;
                let ty = left_var.ty.clone();
                emit_comparison(
                    &mut output,
                    "NATIVE_WHILE_BEGIN",
                    left,
                    operator,
                    &operand,
                    &ty,
                );
            } else {
                let Some((function, args)) = parse_call(condition) else {
                    return Err(diagnostic(
                        source_path,
                        line_number,
                        scope_name,
                        "while condition must compare numeric values or call a Boolean function",
                    ));
                };
                let (link, combined) = if function.contains('.') {
                    resolve_method(
                        &mut output,
                        function,
                        args,
                        &mut values,
                        &native_links,
                        &enum_values,
                        &mut temp_counter,
                    )
                    .map_err(|message| {
                        diagnostic(source_path, line_number, scope_name, &message)
                    })?
                } else {
                    if !imported_functions.contains(function) {
                        return Err(diagnostic(
                            source_path,
                            line_number,
                            scope_name,
                            &format!("Function `{function}` is not imported"),
                        ));
                    }
                    let link = native_links.get(function).ok_or_else(|| {
                        diagnostic(
                            source_path,
                            line_number,
                            scope_name,
                            &format!("Function `{function}` has no native link"),
                        )
                    })?;
                    (link.clone(), args.to_owned())
                };
                if link.return_type != "Boolean" {
                    return Err(diagnostic(
                        source_path,
                        line_number,
                        scope_name,
                        &format!("while condition `{function}` must return Boolean"),
                    ));
                }
                let marker_start = output.len();
                emit_link_call(&mut output, &link, &combined, &enum_values, &values, "").map_err(
                    |message| diagnostic(source_path, line_number, scope_name, &message),
                )?;
                let marker =
                    output[marker_start..].replacen("NATIVE_LINK_ASSIGN", "NATIVE_LOOP_BEGIN", 1);
                output.replace_range(marker_start.., &marker);
            }
            blocks.push('w');
            continue;
        }
        if line == "break;" || line == "break" {
            if !blocks.contains(&'w') {
                return Err(diagnostic(
                    source_path,
                    line_number,
                    scope_name,
                    "`break` must be inside a while loop",
                ));
            }
            output.push('\u{1e}');
            output.push_str("NATIVE_BREAK\n");
            continue;
        }

        let declaration = line.trim_end_matches(';');
        let (prefix, decl) = if let Some(rest) = declaration.strip_prefix("const now into ") {
            ("now", rest)
        } else if let Some(rest) = declaration.strip_prefix("now into ") {
            ("now", rest)
        } else if let Some(rest) = declaration.strip_prefix("late into ") {
            ("late", rest)
        } else if let Some(rest) = declaration.strip_prefix("into ") {
            ("local", rest)
        } else {
            ("", "")
        };
        if !prefix.is_empty() {
            let Some((lhs, rhs)) = decl.split_once('=') else {
                continue;
            };
            let (name, annotation) =
                split_annotation(lhs.trim(), &type_params).map_err(|error| {
                    diagnostic(source_path, line_number, scope_name, &error.to_string())
                })?;
            if name.is_empty() {
                return Err(diagnostic(
                    source_path,
                    line_number,
                    scope_name,
                    "Expected a variable name after `into`",
                ));
            }
            let memory = prefix != "local";
            let (value, ty, native_value) = declaration_value(
                rhs.trim(),
                annotation,
                &mut values,
                &imported_functions,
                &native_links,
                &enum_values,
                &type_params,
                &mut output,
                &name,
                &mut temp_counter,
            )
            .map_err(|message| diagnostic(source_path, line_number, scope_name, &message))?;
            values.insert(
                name,
                Variable {
                    value,
                    ty,
                    memory_backed: memory,
                    allocated: prefix == "now",
                    lazy: prefix == "late",
                    unloaded: false,
                    declared_in_init: in_init,
                    native: native_value,
                },
            );
            continue;
        }
        if let Some((lhs, rhs)) = declaration.split_once('=') {
            let lhs = lhs.trim();
            let rhs = rhs.trim();
            if is_identifier(lhs) && values.contains_key(lhs) && !rhs.starts_with('=') {
                let target_ty = values
                    .get(lhs)
                    .map(|value| value.ty.clone())
                    .unwrap_or(Type::Void);
                let (value, ty, native_value) = declaration_value(
                    rhs,
                    Some(target_ty),
                    &mut values,
                    &imported_functions,
                    &native_links,
                    &enum_values,
                    &type_params,
                    &mut output,
                    lhs,
                    &mut temp_counter,
                )
                .map_err(|message| diagnostic(source_path, line_number, scope_name, &message))?;
                if let Some(variable) = values.get_mut(lhs) {
                    variable.value = value;
                    variable.ty = ty;
                    variable.native = native_value;
                    variable.unloaded = false;
                    variable.allocated = true;
                }
                continue;
            }
        }
        if let Some(rest) = line.strip_prefix("unload ") {
            let name = rest.trim_end_matches(';').trim();
            if let Some(variable) = values.get_mut(name) {
                variable.allocated = false;
                variable.unloaded = true;
            }
            continue;
        }
        if let Some((function, args)) = parse_call(line.trim_end_matches(';')) {
            if function.contains('.') {
                let (receiver, receiver_type) = resolve_receiver(
                    &mut output,
                    function,
                    &mut values,
                    &native_links,
                    &enum_values,
                    &mut temp_counter,
                )
                .map_err(|message| {
                    diagnostic(source_path, line_number, scope_name, &message)
                })?;
                let method = function
                    .split('.')
                    .filter(|segment| !segment.is_empty())
                    .next_back()
                    .unwrap_or_default();
                let key = format!("{}.{}", receiver_type.name(), method);
                if let Some(inner) = user_functions.get(&key) {
                    let inner = inner.clone();
                    inline_user_function(
                        &mut output,
                        &inner,
                        Some(receiver),
                        args,
                        &mut values,
                        &native_links,
                        &enum_values,
                        &mut temp_counter,
                        &user_functions,
                        0,
                    )
                    .map_err(|message| {
                        diagnostic(source_path, line_number, scope_name, &message)
                    })?;
                    continue;
                }
                let link = native_links.get(&key).ok_or_else(|| {
                    diagnostic(
                        source_path,
                        line_number,
                        scope_name,
                        &format!("Undefined method `{key}`"),
                    )
                })?;
                let combined = if args.trim().is_empty() {
                    receiver
                } else {
                    format!("{receiver},{args}")
                };
                emit_link_call(&mut output, link, &combined, &enum_values, &values, "")
                    .map_err(|message| {
                        diagnostic(source_path, line_number, scope_name, &message)
                    })?;
                continue;
            }
            if let Some(inner) = user_functions.get(function) {
                let inner = inner.clone();
                inline_user_function(
                    &mut output,
                    &inner,
                    None,
                    args,
                    &mut values,
                    &native_links,
                    &enum_values,
                    &mut temp_counter,
                    &user_functions,
                    0,
                )
                .map_err(|message| diagnostic(source_path, line_number, scope_name, &message))?;
                continue;
            }
            if imported_functions.contains(function) {
                if let Some(link) = native_links.get(function) {
                    emit_link_call(&mut output, link, args, &enum_values, &values, "").map_err(
                        |message| diagnostic(source_path, line_number, scope_name, &message),
                    )?;
                    continue;
                }
            }
        }
        if let Some((symbol, arg)) = imported_call(line, &imported_functions) {
            let value = arg.trim();
            let (kind, value) = if value.starts_with('"') && value.ends_with('"') {
                ("s", value[1..value.len() - 1].to_owned())
            } else if value.parse::<i32>().is_ok() {
                ("i", value.to_owned())
            } else if let Some((enum_name, variant)) = value.split_once('.') {
                let discriminant = enum_values
                    .get(enum_name)
                    .and_then(|variants| variants.get(variant))
                    .ok_or_else(|| {
                        diagnostic(
                            source_path,
                            line_number,
                            scope_name,
                            &format!("Undefined enum variant `{value}`"),
                        )
                    })?;
                ("i", discriminant.to_string())
            } else if let Some(variable) = values.get(value) {
                ("s", variable.value.clone())
            } else {
                return Err(diagnostic(
                    source_path,
                    line_number,
                    scope_name,
                    &format!("Native call `{symbol}` expects a string or 32-bit integer argument"),
                ));
            };
            output.push('\u{1e}');
            output.push_str("NATIVE\t");
            output.push_str(symbol);
            output.push('\t');
            output.push_str(kind);
            output.push('\t');
            if kind == "i" {
                output.push_str(&value);
            } else {
                for byte in value.bytes() {
                    output.push_str(&format!("{byte:02x}"));
                }
            }
            output.push('\n');
            continue;
        }

        let Some((is_println, rest)) = line
            .strip_prefix("println(")
            .map(|r| (true, r))
            .or_else(|| line.strip_prefix("print(").map(|r| (false, r)))
        else {
            continue;
        };
        let function = if is_println { "println" } else { "print" };
        if !imported_functions.contains(function) {
            return Err(diagnostic(
                source_path,
                line_number,
                scope_name,
                &format!("Function `{function}` is not imported"),
            ));
        }
        let arg = rest.split(')').next().unwrap_or("").trim();
        if let Some((base, field)) = arg.split_once('.') {
            if let Some(variable) = values.get(base) {
                let fields = class_fields.get(&variable.value).ok_or_else(|| {
                    diagnostic(
                        source_path,
                        line_number,
                        scope_name,
                        &format!("Type `{}` has no declared member fields", variable.value),
                    )
                })?;
                let (index, (_, field_type)) = fields
                    .iter()
                    .enumerate()
                    .find(|(_, (name, _))| name == field)
                    .ok_or_else(|| {
                        diagnostic(
                            source_path,
                            line_number,
                            scope_name,
                            &format!("Undefined member `{field}` on `{}`", variable.value),
                        )
                    })?;
                output.push('\u{1e}');
                output.push_str("NATIVE_FIELD_PRINT\t");
                output.push_str(base);
                output.push('\t');
                output.push_str(&variable.value);
                output.push('\t');
                output.push_str(&index.to_string());
                output.push('\t');
                output.push_str(field_type);
                output.push('\t');
                output.push_str(if is_println { "1" } else { "0" });
                output.push('\n');
                continue;
            }
        }
        if arg.starts_with('"') && arg.ends_with('"') {
            output.push_str(&arg[1..arg.len() - 1]);
            if is_println {
                output.push('\n');
            }
            continue;
        }
        if let Some(variable) = values.get_mut(arg) {
            if (!variable.memory_backed && !in_init) || (variable.declared_in_init && !in_init) {
                let scope = if init_has_run {
                    "module scope (after init)"
                } else {
                    "module scope"
                };
                return Err(diagnostic(
                    source_path,
                    line_number,
                    scope,
                    &format!("Variable `{arg}` is out of scope here"),
                ));
            }
            if variable.memory_backed && !variable.allocated && variable.lazy && !variable.unloaded
            {
                variable.allocated = true;
            }
            if variable.memory_backed && !variable.allocated {
                return Err(diagnostic(
                    source_path,
                    line_number,
                    scope_name,
                    &format!("Variable `{arg}` was unloaded"),
                ));
            }
            if variable.native {
                let ty = variable.ty.clone();
                emit_print(&mut output, arg, &ty, is_println);
            } else {
                output.push_str(&variable.value);
                if is_println {
                    output.push('\n');
                }
            }
            continue;
        }
        if let Some(literal) = parse_literal(arg) {
            match literal {
                Literal::Int(number) => {
                    emit_print_const(&mut output, &Type::Int32, &number.to_string(), is_println)
                }
                Literal::Float(number) => {
                    emit_print_const(&mut output, &Type::Float64, &number.to_string(), is_println)
                }
                Literal::Bool(flag) => emit_print_const(
                    &mut output,
                    &Type::Boolean,
                    if flag { "1" } else { "0" },
                    is_println,
                ),
                Literal::String(value) => {
                    output.push_str(&value);
                    if is_println {
                        output.push('\n');
                    }
                }
            }
            continue;
        }
        return Err(diagnostic(
            source_path,
            line_number,
            scope_name,
            &format!("Undefined variable `{arg}`"),
        ));
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn resolve_method(
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
        emit_link_call(output, link, &current, enum_values, &*values, &temporary)?;
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
fn resolve_receiver(
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
        emit_link_call(output, link, &current, enum_values, &*values, &temporary)?;
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
fn inline_user_function(
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
) -> std::result::Result<(), String> {
    if depth > 32 {
        return Err(format!("Recursive call to `{}` is not supported", function.name));
    }
    let arguments = split_argument_list(args);
    if arguments.len() != function.params.len() {
        return Err(format!(
            "Function `{}` expects {} argument(s) but received {}",
            function.name,
            function.params.len(),
            arguments.len()
        ));
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
    for ((param, _), argument) in function.params.iter().zip(arguments.iter()) {
        substitutions.push((param.clone(), argument.trim().to_owned()));
    }
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
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn lower_inline_statement(
    output: &mut String,
    line: &str,
    values: &mut HashMap<String, Variable>,
    native_links: &HashMap<String, imports::NativeLink>,
    enum_values: &HashMap<String, HashMap<String, i32>>,
    temp_counter: &mut usize,
    user_functions: &HashMap<String, UserFunction>,
    depth: usize,
) -> std::result::Result<(), String> {
    let line = line.trim().trim_end_matches(';').trim();
    if line.is_empty() || line == "return" || line == "break" {
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
            );
        }
        let link = native_links
            .get(&key)
            .ok_or_else(|| format!("Undefined method `{key}`"))?;
        let combined = if call_args.trim().is_empty() {
            receiver
        } else {
            format!("{receiver},{call_args}")
        };
        return emit_link_call(output, link, &combined, enum_values, values, "");
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
        );
    }
    let link = native_links
        .get(call)
        .ok_or_else(|| format!("Undefined function `{call}`"))?;
    emit_link_call(output, link, call_args, enum_values, values, "")
}

fn split_argument_list(args: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for ch in args.chars() {
        if in_string {
            current.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                current.push(ch);
            }
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(current.trim().to_owned());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    parts.push(current.trim().to_owned());
    parts.retain(|part| !part.is_empty());
    parts
}

fn apply_substitutions(line: &str, substitutions: &[(String, String)]) -> String {
    let chars = line.chars().collect::<Vec<_>>();
    let mut output = String::new();
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;
    while index < chars.len() {
        let ch = chars[index];
        if in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if ch == '"' {
            in_string = true;
            output.push(ch);
            index += 1;
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = index;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
            {
                index += 1;
            }
            let word = chars[start..index].iter().collect::<String>();
            match substitutions.iter().find(|(name, _)| *name == word) {
                Some((_, replacement)) => output.push_str(replacement),
                None => output.push_str(&word),
            }
            continue;
        }
        output.push(ch);
        index += 1;
    }
    output
}

fn parse_user_functions(source: &str) -> Vec<UserFunction> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut functions = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let clean = strip_line_comment(lines[index]).trim();
        let header = clean
            .strip_prefix("pub fun ")
            .or_else(|| clean.strip_prefix("fun "));
        let Some(header) = header else {
            index += 1;
            continue;
        };
        if !clean.contains('{') {
            index += 1;
            continue;
        }
        let Some(open) = header.find('(') else {
            index += 1;
            continue;
        };
        let name_part = header[..open].trim();
        if name_part.contains('<') {
            index += 1;
            continue;
        }
        let after_open = &header[open + 1..];
        let Some(close) = after_open.find(')') else {
            index += 1;
            continue;
        };
        let params_text = &after_open[..close];
        let (receiver, name) = if let Some((receiver, name)) = name_part.rsplit_once('.') {
            (Some(receiver.trim().to_owned()), name.trim().to_owned())
        } else {
            (None, name_part.to_owned())
        };
        if name.is_empty() {
            index += 1;
            continue;
        }
        let mut params = Vec::new();
        for param in params_text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            if let Some((param_name, param_type)) = param.split_once(':') {
                let ty = Type::parse(param_type.trim(), &HashSet::new()).unwrap_or(Type::Void);
                params.push((param_name.trim().to_owned(), ty));
            } else {
                params.push((param.to_owned(), Type::Void));
            }
        }
        let first_brace = clean.find('{').unwrap_or(clean.len());
        let mut body = Vec::new();
        let inline = clean[first_brace + 1..]
            .trim()
            .trim_end_matches('}')
            .trim();
        if !inline.is_empty() {
            body.push(inline.to_owned());
        }
        let mut depth = brace_delta(clean);
        index += 1;
        while depth > 0 && index < lines.len() {
            let current = strip_line_comment(lines[index]).trim();
            let inside = current.trim_end_matches('}').trim();
            if !inside.is_empty() {
                body.push(inside.to_owned());
            }
            depth += brace_delta(current);
            index += 1;
        }
        functions.push(UserFunction {
            name: match &receiver {
                Some(receiver) => format!("{receiver}.{name}"),
                None => name,
            },
            receiver,
            params,
            body,
        });
    }
    functions
}

#[allow(clippy::too_many_arguments)]
fn declaration_value(
    rhs: &str,
    annotation: Option<Type>,
    values: &mut HashMap<String, Variable>,
    imported_functions: &HashSet<String>,
    native_links: &HashMap<String, imports::NativeLink>,
    enum_values: &HashMap<String, HashMap<String, i32>>,
    type_params: &HashSet<String>,
    output: &mut String,
    assigned_to: &str,
    temp_counter: &mut usize,
) -> std::result::Result<(String, Type, bool), String> {
    if let Some((function, args)) = parse_call(rhs) {
        if function.contains('.') {
            let (link, combined) = resolve_method(
                output,
                function,
                args,
                values,
                native_links,
                enum_values,
                temp_counter,
            )?;
            emit_link_call(output, &link, &combined, enum_values, values, assigned_to)?;
            let ty = match annotation {
                Some(ty) => ty,
                None => Type::parse(&link.return_type, type_params).unwrap_or(Type::Void),
            };
            return Ok((ty.name(), ty, true));
        }
        if imported_functions.contains(function) {
            let link = native_links.get(function).ok_or_else(|| {
                format!("Imported function `{function}` has no #[link] attribute")
            })?;
            emit_link_call(output, link, args, enum_values, values, assigned_to)?;
            let ty = match annotation {
                Some(ty) => ty,
                None => Type::parse(&link.return_type, type_params).unwrap_or(Type::Void),
            };
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
            Literal::String(value) => Ok((value, annotation.unwrap_or(Type::String), false)),
            Literal::Int(number) => {
                let ty = annotation.unwrap_or(Type::Int32);
                emit_assign(output, assigned_to, &ty, "n", &number.to_string());
                Ok((ty.name(), ty, true))
            }
            Literal::Float(number) => {
                let ty = annotation.unwrap_or(Type::Float64);
                emit_assign(output, assigned_to, &ty, "n", &number.to_string());
                Ok((ty.name(), ty, true))
            }
            Literal::Bool(flag) => {
                let ty = annotation.unwrap_or(Type::Boolean);
                emit_assign(output, assigned_to, &ty, "n", if flag { "1" } else { "0" });
                Ok((ty.name(), ty, true))
            }
        };
    }
    if let Some(source) = values.get(rhs) {
        let ty = annotation.unwrap_or_else(|| source.ty.clone());
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

fn arithmetic_operand(right: &str, values: &HashMap<String, Variable>) -> Option<(String, String)> {
    let right = right.trim();
    if let Ok(number) = right.parse::<i64>() {
        return Some(("n".into(), number.to_string()));
    }
    if let Ok(number) = right.parse::<f64>() {
        return Some(("n".into(), number.to_string()));
    }
    if let Some(variable) = values.get(right) {
        if variable.native && variable.ty.is_numeric() {
            return Some(("p".into(), right.to_owned()));
        }
    }
    None
}

fn comparison_operand(right: &str, values: &HashMap<String, Variable>) -> Option<(String, String)> {
    let right = right.trim();
    if let Ok(number) = right.parse::<i64>() {
        return Some(("n".into(), number.to_string()));
    }
    if let Ok(number) = right.parse::<f64>() {
        return Some(("n".into(), number.to_string()));
    }
    if let Some(variable) = values.get(right) {
        if variable.native && variable.ty.is_numeric() {
            return Some(("p".into(), right.to_owned()));
        }
    }
    None
}

fn emit_comparison(
    output: &mut String,
    tag: &str,
    left: &str,
    operator: &str,
    operand: &(String, String),
    ty: &Type,
) {
    output.push('\u{1e}');
    output.push_str(tag);
    output.push('\t');
    output.push_str(left);
    output.push('\t');
    output.push_str(operator);
    output.push('\t');
    output.push_str(&operand.0);
    output.push(':');
    output.push_str(&operand.1);
    output.push('\t');
    output.push_str(&ty.name());
    output.push('\n');
}

fn emit_assign(output: &mut String, target: &str, ty: &Type, kind: &str, value: &str) {
    output.push('\u{1e}');
    output.push_str("NATIVE_ASSIGN\t");
    output.push_str(target);
    output.push('\t');
    output.push_str(&ty.name());
    output.push('\t');
    output.push_str(kind);
    output.push(':');
    output.push_str(value);
    output.push('\n');
}

fn emit_print(output: &mut String, name: &str, ty: &Type, newline: bool) {
    output.push('\u{1e}');
    output.push_str("NATIVE_PRINT\t");
    output.push_str(name);
    output.push('\t');
    output.push_str(&ty.name());
    output.push('\t');
    output.push_str(if newline { "1" } else { "0" });
    output.push('\n');
}

fn emit_print_const(output: &mut String, ty: &Type, value: &str, newline: bool) {
    output.push('\u{1e}');
    output.push_str("NATIVE_PRINT_CONST\t");
    output.push_str(&ty.name());
    output.push('\t');
    output.push_str("n:");
    output.push_str(value);
    output.push('\t');
    output.push_str(if newline { "1" } else { "0" });
    output.push('\n');
}

fn collect_type_params(source: &str) -> HashSet<String> {
    let mut params = HashSet::new();
    for line in source.lines() {
        let line = line.trim();
        let Some((_, rest)) = line.split_once("fun ") else {
            continue;
        };
        let Some(open) = rest.find('<') else {
            continue;
        };
        let Some(close) = rest[open..].find('>') else {
            continue;
        };
        for param in rest[open + 1..open + close].split(',') {
            let param = param.trim();
            if !param.is_empty() {
                params.insert(param.to_owned());
            }
        }
    }
    params
}

fn split_annotation(text: &str, params: &HashSet<String>) -> Result<(String, Option<Type>)> {
    if let Some((name, ty)) = text.split_once(':') {
        Ok((name.trim().to_owned(), Some(Type::parse(ty, params)?)))
    } else {
        Ok((text.trim().to_owned(), None))
    }
}

fn parse_literal(text: &str) -> Option<Literal> {
    let text = text.trim();
    if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
        return Some(Literal::String(decode_string_literal(
            &text[1..text.len() - 1],
        )));
    }
    if text == "true" {
        return Some(Literal::Bool(true));
    }
    if text == "false" {
        return Some(Literal::Bool(false));
    }
    if text.contains('.') {
        if let Ok(value) = text.parse::<f64>() {
            return Some(Literal::Float(value));
        }
    }
    if let Ok(value) = text.parse::<i64>() {
        return Some(Literal::Int(value));
    }
    None
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {}
        _ => return false,
    }
    text.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn parse_enums(
    source: &str,
    source_path: &str,
) -> Result<(
    HashMap<String, HashMap<String, i32>>,
    std::collections::HashSet<usize>,
)> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut enums = HashMap::new();
    let mut consumed = std::collections::HashSet::new();
    let mut index = 0;
    while index < lines.len() {
        let clean = strip_line_comment(lines[index]).trim();
        let Some(rest) = clean.strip_prefix("enum ") else {
            index += 1;
            continue;
        };
        let Some((name, first_body)) = rest.split_once('{') else {
            return Err(diagnostic(
                source_path,
                index + 1,
                "module",
                "Expected `{` in enum declaration",
            ));
        };
        let mut body = first_body.to_owned();
        let mut closed = body.contains('}');
        consumed.insert(index + 1);
        while !closed {
            index += 1;
            if index >= lines.len() {
                break;
            }
            let part = strip_line_comment(lines[index]).trim();
            body.push(' ');
            body.push_str(part);
            consumed.insert(index + 1);
            closed = part.contains('}');
        }
        if !closed {
            return Err(diagnostic(
                source_path,
                index + 1,
                "module",
                "Expected `}` in enum declaration",
            ));
        }
        let body = body.split_once('}').map_or(body.as_str(), |(body, _)| body);
        let name = name.trim();
        if name.is_empty() || !name.chars().next().is_some_and(char::is_uppercase) {
            return Err(diagnostic(
                source_path,
                index + 1,
                "module",
                "Enum names must start with an uppercase letter",
            ));
        }
        let mut variants = HashMap::new();
        let mut next = 0i32;
        for variant in body.split(',').map(str::trim).filter(|v| !v.is_empty()) {
            let (variant_name, value) = if let Some((variant_name, value)) = variant.split_once('=')
            {
                (
                    variant_name.trim(),
                    value.trim().parse::<i32>().map_err(|_| {
                        diagnostic(
                            source_path,
                            index + 1,
                            "module",
                            "Enum discriminants must be 32-bit integers",
                        )
                    })?,
                )
            } else {
                (variant, next)
            };
            if !variant_name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase() || c == '_')
            {
                return Err(diagnostic(
                    source_path,
                    index + 1,
                    "module",
                    "Enum variant names must start with an uppercase letter",
                ));
            }
            if variants.insert(variant_name.to_owned(), value).is_some() {
                return Err(diagnostic(
                    source_path,
                    index + 1,
                    "module",
                    &format!("Duplicate enum variant `{variant_name}`"),
                ));
            }
            next = value.saturating_add(1);
        }
        if enums.insert(name.to_owned(), variants).is_some() {
            return Err(diagnostic(
                source_path,
                index + 1,
                "module",
                &format!("Duplicate enum `{name}`"),
            ));
        }
        index += 1;
    }
    Ok((enums, consumed))
}

fn strip_line_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quoted = false;
    let mut escaped = false;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if quoted => escaped = !escaped,
            b'"' if !escaped => quoted = !quoted,
            b'/' if !quoted && bytes.get(index + 1) == Some(&b'/') => return &line[..index],
            _ if escaped => escaped = false,
            _ => {}
        }
        if bytes[index] != b'\\' {
            escaped = false;
        }
        index += 1;
    }
    line
}

fn brace_delta(line: &str) -> i32 {
    let line = strip_line_comment(line);
    let mut delta = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for ch in line.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => delta += 1,
            '}' => delta -= 1,
            _ => {}
        }
    }
    delta
}

fn parse_call(expression: &str) -> Option<(&str, &str)> {
    let (name, args) = expression.split_once('(')?;
    Some((name.trim(), args.strip_suffix(')')?))
}

fn parse_arithmetic(expression: &str) -> Option<(&str, char, &str)> {
    if expression.starts_with('"') {
        return None;
    }
    for op in ['+', '-', '*', '/'] {
        if let Some((left, right)) = expression.split_once(op) {
            let left = left.trim();
            let right = right.trim();
            if !left.is_empty() && !right.is_empty() && !right.contains(op) {
                return Some((left, op, right));
            }
        }
    }
    None
}

fn parse_comparison(expression: &str) -> Option<(&str, &str, &str)> {
    for operator in [">=", "<=", "==", "!=", ">", "<"] {
        if let Some((left, right)) = expression.split_once(operator) {
            let left = left.trim();
            let right = right.trim();
            if !left.is_empty() && !right.is_empty() {
                return Some((left, operator, right));
            }
        }
    }
    None
}

fn parse_classes(source: &str) -> ClassFields {
    let lines = source.lines().collect::<Vec<_>>();
    let mut classes = HashMap::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim();
        let Some((prefix, _)) = line.split_once("() {") else {
            index += 1;
            continue;
        };
        let name = prefix.strip_prefix("pub ").unwrap_or(prefix).trim();
        if name.is_empty() {
            index += 1;
            continue;
        }
        if line
            .split_once("() {")
            .is_some_and(|(_, body)| body.contains('}'))
        {
            classes.insert(name.to_owned(), Vec::new());
            index += 1;
            continue;
        }
        let mut fields = Vec::new();
        index += 1;
        while index < lines.len() && !lines[index].trim().starts_with('}') {
            let declaration = lines[index].trim().trim_end_matches(';');
            if let Some((field, ty)) = declaration.split_once(':') {
                fields.push((field.trim().to_owned(), ty.trim().to_owned()));
            }
            index += 1;
        }
        classes.insert(name.to_owned(), fields);
        index += 1;
    }
    classes
}

fn emit_link_call(
    output: &mut String,
    link: &imports::NativeLink,
    args: &str,
    enums: &HashMap<String, HashMap<String, i32>>,
    values: &HashMap<String, Variable>,
    assigned_to: &str,
) -> std::result::Result<(), String> {
    output.push('\u{1e}');
    output.push_str("NATIVE_LINK_ASSIGN\t");
    output.push_str(&link.package_alias);
    output.push('\t');
    output.push_str(&link.language);
    output.push('\t');
    output.push_str(&link.source_path);
    output.push('\t');
    output.push_str(&link.target);
    output.push('\t');
    output.push_str(&link.return_type);
    output.push('\t');
    output.push_str(assigned_to);
    for arg in args.split(',').map(str::trim).filter(|arg| !arg.is_empty()) {
        let (kind, value) = if arg.starts_with('"') && arg.ends_with('"') {
            ("s", decode_string_literal(&arg[1..arg.len() - 1]))
        } else if let Ok(number) = arg.parse::<i32>() {
            ("i", number.to_string())
        } else if let Some((enum_name, variant)) = arg.split_once('.') {
            let value = enums
                .get(enum_name)
                .and_then(|variants| variants.get(variant))
                .ok_or_else(|| format!("Undefined enum variant `{arg}`"))?;
            ("i", value.to_string())
        } else if let Some(variable) = values.get(arg) {
            if variable.native {
                ("p", arg.to_owned())
            } else {
                ("s", variable.value.clone())
            }
        } else {
            return Err(format!("Unsupported native argument `{arg}`"));
        };
        output.push('\t');
        output.push_str(kind);
        output.push(':');
        if kind == "i" || kind == "p" {
            output.push_str(&value);
        } else {
            for byte in value.bytes() {
                output.push_str(&format!("{byte:02x}"));
            }
        }
    }
    output.push('\n');
    Ok(())
}

fn decode_string_literal(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some('t') => output.push('\t'),
            Some('"') => output.push('"'),
            Some('\\') => output.push('\\'),
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
            None => output.push('\\'),
        }
    }
    output
}

fn imported_call<'a>(
    line: &'a str,
    imported_functions: &std::collections::HashSet<String>,
) -> Option<(&'a str, &'a str)> {
    let call = line.strip_suffix(';').unwrap_or(line).trim();
    let (name, args) = call.split_once('(')?;
    let name = name.trim();
    let args = args.strip_suffix(')')?;
    if matches!(name, "print" | "println")
        || !imported_functions.contains(name)
        || args.contains(',')
    {
        return None;
    }
    Some((name, args))
}
