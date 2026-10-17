use anyhow::{Result, anyhow};
use std::collections::HashSet;

use crate::types::Type;

pub fn diagnostic(path: &str, line: usize, scope: &str, message: &str) -> anyhow::Error {
    anyhow!("{message}\n\nInto language backtrace:\n  0: {scope} at {path}:{line}")
}

pub fn split_argument_list(args: &str) -> Vec<String> {
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

pub fn apply_substitutions(line: &str, substitutions: &[(String, String)]) -> String {
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

pub fn collect_type_params(source: &str) -> HashSet<String> {
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

pub fn split_annotation(text: &str, params: &HashSet<String>) -> Result<(String, Option<Type>)> {
    if let Some((name, ty)) = text.split_once(':') {
        Ok((name.trim().to_owned(), Some(Type::parse(ty, params)?)))
    } else {
        Ok((text.trim().to_owned(), None))
    }
}

pub fn boolean_literal(text: &str) -> Option<bool> {
    match text.trim() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

pub fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {}
        _ => return false,
    }
    text.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// The class name underlying a (possibly generic) type spelling, e.g.
/// `List<Int32>` becomes `List`. Non-generic names are returned unchanged.
pub fn base_type_name(name: &str) -> &str {
    name.split('<').next().unwrap_or(name).trim()
}

/// Consume a leading identifier token. The token ends at the first character
/// that cannot appear in an identifier, so the boundary is defined by the
/// identifier alphabet rather than by surrounding whitespace. Returns the
/// identifier and the remaining text.
pub fn take_identifier(text: &str) -> Option<(&str, &str)> {
    let mut end = 0;
    for (index, ch) in text.char_indices() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            end = index + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    let name = &text[..end];
    let first = name.chars().next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    Some((name, &text[end..]))
}

/// Read the next identifier token, ignoring any leading whitespace. Returns the
/// word and the text after it.
fn next_word(text: &str) -> Option<(&str, &str)> {
    take_identifier(text.trim_start())
}

/// Match a sequence of keyword words, ignoring the amount of whitespace between
/// them. Returns the text following the final word.
fn match_words<'a>(text: &'a str, words: &[&str]) -> Option<&'a str> {
    let mut rest = text;
    for word in words {
        let (actual, after) = next_word(rest)?;
        if actual != *word {
            return None;
        }
        rest = after;
    }
    Some(rest)
}

/// Recognise an `into`-family declaration prefix, returning the storage keyword
/// (`local`/`now`/`late`), whether the declaration marks a pointer with `*`, and
/// the remainder beginning at the variable name.
///
/// Keywords are matched as a sequence of identifier words, so the number of
/// spaces between them is irrelevant, and the variable name is read with
/// [`take_identifier`], so identifiers such as `intoVariable` are never mistaken
/// for a declaration.
pub fn strip_declaration_prefix(text: &str) -> Option<(&'static str, bool, &str)> {
    const KEYWORDS: [(&[&str], &str); 4] = [
        (&["const", "now", "into"], "now"),
        (&["now", "into"], "now"),
        (&["late", "into"], "late"),
        (&["into"], "local"),
    ];
    for (words, storage) in KEYWORDS {
        let Some(rest) = match_words(text, words) else {
            continue;
        };
        let rest = rest.trim_start();
        let (is_pointer, rest) = match rest.strip_prefix('*') {
            Some(rest) => (true, rest.trim_start()),
            None => (false, rest),
        };
        take_identifier(rest)?;
        return Some((storage, is_pointer, rest));
    }
    None
}

/// Remove a leading `keyword` from `text` when it is followed by whitespace,
/// returning the remainder with leading whitespace removed.
pub fn strip_keyword<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(keyword)?;
    rest.starts_with(char::is_whitespace)
        .then(|| rest.trim_start())
}

pub fn strip_line_comment(line: &str) -> &str {
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

pub fn brace_delta(line: &str) -> i32 {
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

pub fn parse_call(expression: &str) -> Option<(&str, &str)> {
    let (name, args) = expression.split_once('(')?;
    Some((name.trim(), args.strip_suffix(')')?))
}

pub fn parse_arithmetic(expression: &str) -> Option<(&str, char, &str)> {
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

pub fn parse_comparison(expression: &str) -> Option<(&str, &str, &str)> {
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

pub fn decode_string_literal(value: &str) -> String {
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

pub fn imported_call<'a>(
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
