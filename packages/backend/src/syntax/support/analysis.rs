use std::collections::HashMap;

use anyhow::Result;

use crate::imports;
use crate::syntax::context::{ClassFields, UserFunction};
use crate::syntax::support::text::{brace_delta, diagnostic, strip_line_comment};

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

pub fn parse_classes(source: &str) -> ClassFields {
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

pub fn parse_user_functions(source: &str) -> Vec<UserFunction> {
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
            params,
            body,
        });
    }
    functions
}
