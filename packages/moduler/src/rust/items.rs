use super::text::{brace_delta, is_identifier, strip_keyword, strip_line_comment};

/// The line span occupied by a class declaration, including its header.
#[derive(Debug, Clone)]
pub struct ClassSpan {
    pub name: String,
    pub is_public: bool,
    pub start: usize,
    pub end: usize,
}

/// Locate every `Name() { ... }` declaration and its body extent.
pub fn scan_classes(source: &str) -> Vec<ClassSpan> {
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
