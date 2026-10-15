use anyhow::{Result, anyhow};
use std::{collections::HashMap, path::Path};

use crate::{imports, lexer::tokenize, validation};

/// Parse the currently supported statement subset and lower it to output.
pub(super) fn lower(source: &str, packages_dir: Option<&Path>) -> Result<String> {
    lower_named(source, packages_dir, "<source>")
}

struct Variable {
    value: String,
    memory_backed: bool,
    allocated: bool,
    lazy: bool,
    unloaded: bool,
    declared_in_init: bool,
    native: bool,
}

type ClassFields = HashMap<String, Vec<(String, String)>>;

fn diagnostic(path: &str, line: usize, scope: &str, message: &str) -> anyhow::Error {
    anyhow!("{message}\n\nInto language backtrace:\n  0: {scope} at {path}:{line}")
}

pub(super) fn lower_named(source: &str, packages_dir: Option<&Path>, source_path: &str) -> Result<String> {
    let _tokens = tokenize(source)?;
    validation::function_scopes(source, source_path)?;
    let imported_functions = if let Some(packages_dir) = packages_dir {
        imports::resolve(source, packages_dir)?
    } else {
        Default::default()
    };
    let native_links = if let Some(packages_dir) = packages_dir {
        imports::resolve_links(source, packages_dir)?
    } else { HashMap::new() };

    let mut values: HashMap<String, Variable> = HashMap::new();
    let (mut enum_values, enum_lines) = parse_enums(source, source_path)?;
    let module_sources = if let Some(packages_dir) = packages_dir {
        imports::imported_module_sources(source, packages_dir)?
    } else { Vec::new() };
    for module in &module_sources {
            let (module_enums, _) = parse_enums(&module, source_path)?;
            for (name, variants) in module_enums { enum_values.entry(name).or_insert(variants); }
    }
    let class_fields = module_sources.iter().flat_map(|module| parse_classes(module)).collect::<HashMap<_, _>>();
    let mut output = String::new();
    for (class, fields) in &class_fields {
        output.push('\u{1e}');
        output.push_str("NATIVE_CLASS\t");
        output.push_str(class);
        for (field, ty) in fields {
            output.push('\t'); output.push_str(field); output.push(':'); output.push_str(ty);
        }
        output.push('\n');
    }
    let mut in_init = false;
    let mut init_has_run = false;
    let mut blocks = Vec::new();

    for (index, raw) in source.lines().enumerate() {
        let line_number = index + 1;
        let code = strip_line_comment(raw);
        let line = code.trim();
        if line.starts_with("import ") || line.is_empty() { continue; }
        if enum_lines.contains(&line_number) { continue; }
        if line.starts_with("init") && line.contains('{') { in_init = true; init_has_run = true; blocks.push('i'); continue; }
        if line == "} else {" {
            if blocks.last() != Some(&'f') { return Err(diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, "`else` must follow an `if` block")); }
            output.push('\u{1e}'); output.push_str("NATIVE_IF_ELSE\n");
            continue;
        }
        if line == "}" {
            match blocks.pop() {
                Some('w') => { output.push('\u{1e}'); output.push_str("NATIVE_LOOP_END\n"); }
                Some('f') => { output.push('\u{1e}'); output.push_str("NATIVE_IF_END\n"); }
                Some('i') => in_init = false,
                _ => return Err(diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, "Unexpected closing brace")),
            }
            continue;
        }
        if let Some(condition) = line.strip_prefix("if ").and_then(|v| v.strip_suffix('{')).map(str::trim) {
            let condition = condition.strip_prefix('(').and_then(|v| v.strip_suffix(')')).unwrap_or(condition).trim();
            let (variable, operator, literal) = parse_comparison(condition).ok_or_else(|| diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, "if condition must compare a value with an integer"))?;
            if !values.get(variable).is_some_and(|value| value.native && value.value == "Int32") || literal.parse::<i32>().is_err() {
                return Err(diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, "if condition requires an Int32 value and integer literal"));
            }
            output.push('\u{1e}'); output.push_str("NATIVE_IF_BEGIN\t"); output.push_str(variable);
            output.push('\t'); output.push_str(operator); output.push('\t'); output.push_str(literal); output.push('\n');
            blocks.push('f');
            continue;
        }
        if let Some(condition) = line.strip_prefix("while ").and_then(|v| v.strip_suffix('{')).map(str::trim) {
            let condition = condition.strip_prefix('(').and_then(|v| v.strip_suffix(')')).unwrap_or(condition).trim();
            let Some((function, args)) = parse_call(condition) else {
                return Err(diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, "while condition must be a function call returning Boolean"));
            };
            if !imported_functions.contains(function) {
                return Err(diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, &format!("Function `{function}` is not imported")));
            }
            let link = native_links.get(function).ok_or_else(|| diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, &format!("Function `{function}` has no native link")))?;
            if link.return_type != "Boolean" {
                return Err(diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, &format!("while condition `{function}` must return Boolean")));
            }
            let marker_start = output.len();
            emit_link_call(&mut output, link, args, &enum_values, &values, "")
                .map_err(|message| diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, &message))?;
            let marker = output[marker_start..].replacen("NATIVE_LINK_ASSIGN", "NATIVE_LOOP_BEGIN", 1);
            output.replace_range(marker_start.., &marker);
            blocks.push('w');
            continue;
        }

        let declaration = line.trim_end_matches(';');
        let (prefix, decl) = if let Some(rest) = declaration.strip_prefix("const now into ") { ("now", rest) }
            else if let Some(rest) = declaration.strip_prefix("now into ") { ("now", rest) }
            else if let Some(rest) = declaration.strip_prefix("late into ") { ("late", rest) }
            else if let Some(rest) = declaration.strip_prefix("into ") { ("local", rest) }
            else { ("", "") };
        if !prefix.is_empty() {
            if let Some((name, rhs)) = decl.split_once('=') {
                let rhs = rhs.trim();
                let mut native_value = false;
                let value = if let Some((function, args)) = parse_call(rhs) {
                    if imported_functions.contains(function) {
                        if let Some(link) = native_links.get(function) {
                            emit_link_call(&mut output, link, args, &enum_values, &values, name.trim())
                                .map_err(|message| diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, &message))?;
                            native_value = true;
                            link.return_type.clone()
                        } else {
                            return Err(diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, &format!("Imported function `{function}` has no #[link] attribute")));
                        }
                    } else { rhs.trim_matches('"').to_owned() }
                } else if let Some((left, op, right)) = parse_arithmetic(rhs) {
                    let Some(left_value) = values.get(left).filter(|value| value.native && value.value == "Int32") else {
                        return Err(diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, "Arithmetic requires an Int32 value"));
                    };
                    let right_operand = if let Ok(number) = right.parse::<i32>() { format!("i:{number}") }
                        else if values.get(right).is_some_and(|value| value.native && value.value == "Int32") { format!("p:{right}") }
                        else { return Err(diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, "Arithmetic requires an Int32 operand")); };
                    let operation = match op { '+' => "add", '-' => "sub", '*' => "mul", '/' => "div", _ => unreachable!() };
                    output.push('\u{1e}'); output.push_str("NATIVE_ARITH\t"); output.push_str(name.trim());
                    output.push('\t'); output.push_str(left); output.push('\t'); output.push_str(operation);
                    output.push('\t'); output.push_str(&right_operand); output.push('\n');
                    let _ = left_value;
                    native_value = true;
                    "Int32".to_owned()
                } else { rhs.trim_matches('"').to_owned() };
                let memory = prefix != "local";
                values.insert(name.trim().to_owned(), Variable {
                    value,
                    memory_backed: memory,
                    allocated: prefix == "now",
                    lazy: prefix == "late",
                    unloaded: false,
                    declared_in_init: in_init,
                    native: native_value,
                });
            }
            continue;
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
            if imported_functions.contains(function) {
                if let Some(link) = native_links.get(function) {
                    emit_link_call(&mut output, link, args, &enum_values, &values, "")
                        .map_err(|message| diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, &message))?;
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
                let discriminant = enum_values.get(enum_name).and_then(|variants| variants.get(variant))
                    .ok_or_else(|| diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, &format!("Undefined enum variant `{value}`")))?;
                ("i", discriminant.to_string())
            } else if let Some(variable) = values.get(value) {
                ("s", variable.value.clone())
            } else {
                return Err(diagnostic(source_path, line_number, if in_init { "init" } else { "module" },
                    &format!("Native call `{symbol}` expects a string or 32-bit integer argument")));
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
                for byte in value.bytes() { output.push_str(&format!("{byte:02x}")); }
            }
            output.push('\n');
            continue;
        }

        let Some((is_println, rest)) = line.strip_prefix("println(").map(|r| (true, r))
            .or_else(|| line.strip_prefix("print(").map(|r| (false, r))) else { continue; };
        let function = if is_println { "println" } else { "print" };
        if !imported_functions.contains(function) {
            return Err(diagnostic(source_path, line_number, if in_init { "init" } else { "module" },
                &format!("Function `{function}` is not imported")));
        }
        let arg = rest.split(')').next().unwrap_or("").trim();
        if let Some((base, field)) = arg.split_once('.') {
            if let Some(variable) = values.get(base) {
                let fields = class_fields.get(&variable.value).ok_or_else(|| diagnostic(
                    source_path, line_number, if in_init { "init" } else { "module" },
                    &format!("Type `{}` has no declared member fields", variable.value),
                ))?;
                let (index, (_, field_type)) = fields.iter().enumerate().find(|(_, (name, _))| name == field)
                    .ok_or_else(|| diagnostic(source_path, line_number, if in_init { "init" } else { "module" }, &format!("Undefined member `{field}` on `{}`", variable.value)))?;
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
        let (value, newline) = if arg.starts_with('"') && arg.ends_with('"') {
            (arg[1..arg.len()-1].to_owned(), is_println)
        } else if let Some(variable) = values.get_mut(arg) {
            if (!variable.memory_backed && !in_init) || (variable.declared_in_init && !in_init) {
                let scope = if init_has_run { "module scope (after init)" } else { "module scope" };
                return Err(diagnostic(source_path, line_number, scope,
                    &format!("Variable `{arg}` is out of scope here")));
            }
            if variable.memory_backed && !variable.allocated && variable.lazy && !variable.unloaded {
                variable.allocated = true;
            }
            if variable.memory_backed && !variable.allocated {
                return Err(diagnostic(source_path, line_number, if in_init { "init" } else { "module" },
                    &format!("Variable `{arg}` was unloaded")));
            }
            (variable.value.clone(), is_println)
        } else {
            let scope = if in_init { "init" } else { "module" };
            return Err(diagnostic(source_path, line_number, scope,
                &format!("Undefined variable `{arg}`")));
        };
        output.push_str(&value);
        if newline { output.push('\n'); }
    }
    Ok(output)
}

fn parse_enums(source: &str, source_path: &str) -> Result<(HashMap<String, HashMap<String, i32>>, std::collections::HashSet<usize>)> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut enums = HashMap::new();
    let mut consumed = std::collections::HashSet::new();
    let mut index = 0;
    while index < lines.len() {
        let clean = strip_line_comment(lines[index]).trim();
        let Some(rest) = clean.strip_prefix("enum ") else { index += 1; continue; };
        let Some((name, first_body)) = rest.split_once('{') else {
            return Err(diagnostic(source_path, index + 1, "module", "Expected `{` in enum declaration"));
        };
        let mut body = first_body.to_owned();
        let mut closed = body.contains('}');
        consumed.insert(index + 1);
        while !closed {
            index += 1;
            if index >= lines.len() { break; }
            let part = strip_line_comment(lines[index]).trim();
            body.push(' ');
            body.push_str(part);
            consumed.insert(index + 1);
            closed = part.contains('}');
        }
        if !closed { return Err(diagnostic(source_path, index + 1, "module", "Expected `}` in enum declaration")); }
        let body = body.split_once('}').map_or(body.as_str(), |(body, _)| body);
        let name = name.trim();
        if name.is_empty() || !name.chars().next().is_some_and(char::is_uppercase) {
            return Err(diagnostic(source_path, index + 1, "module", "Enum names must start with an uppercase letter"));
        }
        let mut variants = HashMap::new();
        let mut next = 0i32;
        for variant in body.split(',').map(str::trim).filter(|v| !v.is_empty()) {
            let (variant_name, value) = if let Some((variant_name, value)) = variant.split_once('=') {
                (variant_name.trim(), value.trim().parse::<i32>().map_err(|_| diagnostic(source_path, index + 1, "module", "Enum discriminants must be 32-bit integers"))?)
            } else { (variant, next) };
            if !variant_name.chars().next().is_some_and(|c| c.is_ascii_uppercase() || c == '_') {
                return Err(diagnostic(source_path, index + 1, "module", "Enum variant names must start with an uppercase letter"));
            }
            if variants.insert(variant_name.to_owned(), value).is_some() {
                return Err(diagnostic(source_path, index + 1, "module", &format!("Duplicate enum variant `{variant_name}`")));
            }
            next = value.saturating_add(1);
        }
        if enums.insert(name.to_owned(), variants).is_some() {
            return Err(diagnostic(source_path, index + 1, "module", &format!("Duplicate enum `{name}`")));
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
        if bytes[index] != b'\\' { escaped = false; }
        index += 1;
    }
    line
}

fn parse_call(expression: &str) -> Option<(&str, &str)> {
    let (name, args) = expression.split_once('(')?;
    Some((name.trim(), args.strip_suffix(')')?))
}

fn parse_arithmetic(expression: &str) -> Option<(&str, char, &str)> {
    if expression.starts_with('"') { return None; }
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
            if !left.is_empty() && !right.is_empty() { return Some((left, operator, right)); }
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
        let Some((prefix, _)) = line.split_once("() {") else { index += 1; continue; };
        let name = prefix.strip_prefix("pub ").unwrap_or(prefix).trim();
        if name.is_empty() { index += 1; continue; }
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
            let value = enums.get(enum_name).and_then(|variants| variants.get(variant))
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
        if kind == "i" || kind == "p" { output.push_str(&value); }
        else { for byte in value.bytes() { output.push_str(&format!("{byte:02x}")); } }
    }
    output.push('\n');
    Ok(())
}

fn decode_string_literal(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' { output.push(ch); continue; }
        match chars.next() {
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some('t') => output.push('\t'),
            Some('"') => output.push('"'),
            Some('\\') => output.push('\\'),
            Some(other) => { output.push('\\'); output.push(other); }
            None => output.push('\\'),
        }
    }
    output
}

fn imported_call<'a>(line: &'a str, imported_functions: &std::collections::HashSet<String>) -> Option<(&'a str, &'a str)> {
    let call = line.strip_suffix(';').unwrap_or(line).trim();
    let (name, args) = call.split_once('(')?;
    let name = name.trim();
    let args = args.strip_suffix(')')?;
    if matches!(name, "print" | "println") || !imported_functions.contains(name) || args.contains(',') { return None; }
    Some((name, args))
}
