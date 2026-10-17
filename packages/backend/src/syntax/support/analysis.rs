use std::collections::HashMap;

use anyhow::Result;

use crate::imports;
use crate::syntax::context::{ClassFields, FieldDef, UserFunction};
use crate::syntax::support::text::{
    brace_delta, diagnostic, is_identifier, strip_keyword, strip_line_comment,
};

/// The line span occupied by a class declaration, including its header.
struct ClassSpan {
    name: String,
    is_public: bool,
    start: usize,
    end: usize,
}

/// Locate every `Name() { ... }` declaration and its body extent.
fn scan_classes(source: &str) -> Vec<ClassSpan> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut spans = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let code = strip_line_comment(lines[index]).trim();
        let Some((prefix, _)) = code.split_once("() {") else {
            index += 1;
            continue;
        };
        let (is_public, name) = match strip_keyword(prefix, "pub") {
            Some(rest) => (true, rest.trim()),
            None => (false, prefix.trim()),
        };
        if name.is_empty() || !is_identifier(name) {
            index += 1;
            continue;
        }
        let start = index;
        let mut depth = brace_delta(code);
        index += 1;
        while depth > 0 && index < lines.len() {
            depth += brace_delta(strip_line_comment(lines[index]));
            index += 1;
        }
        spans.push(ClassSpan {
            name: name.to_owned(),
            is_public,
            start,
            end: index,
        });
    }
    spans
}

/// Collect the names a module exposes publicly.
///
/// Only declarations carrying the `pub` modifier are visible to importers.
/// Free declarations contribute their bare name and class members contribute
/// `Class.member`; `pub` classes and `pub macro`s are included as well.
pub fn public_items(source: &str) -> std::collections::HashSet<String> {
    let lines = source.lines().collect::<Vec<_>>();
    let spans = scan_classes(source);
    let mut items = std::collections::HashSet::new();
    for span in &spans {
        if span.is_public {
            items.insert(span.name.clone());
        }
    }
    for (index, raw) in lines.iter().enumerate() {
        let line = strip_line_comment(raw).trim();
        let Some(rest) = strip_keyword(line, "pub") else {
            continue;
        };
        if let Some(rest) = strip_keyword(rest, "macro") {
            let name = rest.split_whitespace().next().unwrap_or("");
            if is_identifier(name) {
                items.insert(name.to_owned());
            }
            continue;
        }
        let rest = strip_keyword(rest, "static").unwrap_or(rest);
        let rest = strip_keyword(rest, "c").unwrap_or(rest);
        let rest = strip_keyword(rest, "cpp").unwrap_or(rest);
        let Some(header) = strip_keyword(rest, "fun") else {
            continue;
        };
        let name = header.split('(').next().unwrap_or("").trim();
        if !is_identifier(name) {
            continue;
        }
        let owner = spans
            .iter()
            .find(|span| index > span.start && index < span.end)
            .map(|span| span.name.clone());
        match owner {
            Some(class) => {
                items.insert(format!("{class}.{name}"));
            }
            None => {
                items.insert(name.to_owned());
            }
        }
    }
    items
}

pub fn parse_enums(
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

/// Parse `pub into: <Type> <name> = <default>;` fields from every class body.
///
/// The legacy `name: Type;` field syntax is rejected with a diagnostic.
pub fn parse_classes(source: &str, source_path: &str) -> Result<ClassFields> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut classes = HashMap::new();
    for span in scan_classes(source) {
        let mut fields = Vec::new();
        let mut depth = brace_delta(strip_line_comment(lines[span.start]));
        let mut index = span.start + 1;
        while depth > 0 && index < lines.len() {
            let line = strip_line_comment(lines[index]).trim();
            if depth == 1 {
                let field_spec = strip_keyword(line, "pub").and_then(|rest| {
                    let (is_static, rest) = match strip_keyword(rest, "static") {
                        Some(rest) => (true, rest),
                        None => (false, rest),
                    };
                    rest.strip_prefix("into:").map(|spec| (is_static, spec))
                });
                if let Some((is_static, rest)) = field_spec {
                    let spec = rest.trim().trim_end_matches(';').trim();
                    let (declaration, default) = match spec.split_once('=') {
                        Some((declaration, default)) => {
                            (declaration.trim(), Some(default.trim().to_owned()))
                        }
                        None => (spec, None),
                    };
                    let mut parts = declaration.split_whitespace();
                    let ty = parts.next().unwrap_or("");
                    let name = parts.next().unwrap_or("");
                    if ty.is_empty() || !is_identifier(name) || parts.next().is_some() {
                        return Err(diagnostic(
                            source_path,
                            index + 1,
                            "module",
                            &format!(
                                "Expected class field 'pub into: <Type> <name> = <default>;' in `{}`",
                                span.name
                            ),
                        ));
                    }
                    fields.push(FieldDef {
                        name: name.to_owned(),
                        ty: ty.to_owned(),
                        default,
                        is_static,
                    });
                } else if !line.is_empty()
                    && !line.starts_with('}')
                    && !line.contains("fun ")
                    && !line.starts_with("#[")
                {
                    return Err(diagnostic(
                        source_path,
                        index + 1,
                        "module",
                        &format!(
                            "Class `{}` fields must be declared as 'pub into: <Type> <name> = <default>;'",
                            span.name
                        ),
                    ));
                }
            }
            depth += brace_delta(line);
            index += 1;
        }
        classes.insert(span.name, fields);
    }
    Ok(classes)
}

/// Parse Into-bodied functions.
///
/// Functions declared inside a class body are methods of that class; their
/// stored name is `Class.method`. The legacy top-level `pub fun Class.method`
/// form is rejected.
pub fn parse_user_functions(source: &str, source_path: &str) -> Result<Vec<UserFunction>> {
    let lines = source.lines().collect::<Vec<_>>();
    let spans = scan_classes(source);
    let mut functions = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let clean = strip_line_comment(lines[index]).trim();
        let without_visibility = strip_keyword(clean, "pub").unwrap_or(clean);
        let (is_static, rest) = match strip_keyword(without_visibility, "static") {
            Some(rest) => (true, rest),
            None => (false, without_visibility),
        };
        let Some(header) = strip_keyword(rest, "fun") else {
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
        let enclosing = spans
            .iter()
            .find(|span| index > span.start && index < span.end)
            .map(|span| span.name.clone());
        let (receiver, name) = match enclosing {
            Some(class) => (Some(class), name_part.to_owned()),
            None => {
                if name_part.contains('.') {
                    return Err(diagnostic(
                        source_path,
                        index + 1,
                        "module",
                        "Methods must be declared inside their class body as 'pub fun <name>(...)'",
                    ));
                }
                (None, name_part.to_owned())
            }
        };
        if name.is_empty() {
            index += 1;
            continue;
        }
        let params = imports::parse_params(params_text);
        let first_brace = clean.find('{').unwrap_or(clean.len());
        let mut body = Vec::new();
        let inline = clean[first_brace + 1..].trim().trim_end_matches('}').trim();
        if !inline.is_empty() {
            body.push(inline.to_owned());
        }
        let mut depth = brace_delta(clean);
        index += 1;
        while depth > 0 && index < lines.len() {
            let current = strip_line_comment(lines[index]).trim();
            let next_depth = depth + brace_delta(current);
            if next_depth == 0 {
                depth = 0;
                index += 1;
                break;
            }
            if !current.is_empty() {
                body.push(current.to_owned());
            }
            depth = next_depth;
            index += 1;
        }
        functions.push(UserFunction {
            name: match &receiver {
                Some(receiver) => format!("{receiver}.{name}"),
                None => name,
            },
            receiver,
            is_static,
            params,
            body,
        });
    }
    Ok(functions)
}
