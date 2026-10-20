use std::collections::HashMap;

use tokenizer::Tokens;
use types::TokenKind;

const MARKER: &str = "///";

/// The `///` comments of a source text, by the line of the token they sit above.
pub struct DocComments {
    by_line: HashMap<usize, String>,
    lines: Vec<String>,
}

impl DocComments {
    pub fn new(source: &str) -> Self {
        let lines = source.lines().map(str::to_owned).collect();
        let mut by_line = HashMap::new();
        let Ok(tokens) = Tokens::with_comments(source) else {
            return Self { by_line, lines };
        };
        let mut group: Vec<(usize, String)> = Vec::new();
        let mut previous_line = 0;
        for token in tokens.iter() {
            let line = token.span.line;
            match &token.kind {
                TokenKind::Comment(text) if text.starts_with(MARKER) && line > previous_line => {
                    if group.last().is_some_and(|(last, _)| *last + 1 != line) {
                        group.clear();
                    }
                    let body = &text[MARKER.len()..];
                    group.push((line, body.strip_prefix(' ').unwrap_or(body).to_owned()));
                }
                TokenKind::Comment(_) => group.clear(),
                _ => {
                    if let Some((last, _)) = group.last()
                        && *last + 1 == line
                    {
                        let text: Vec<&str> = group.iter().map(|(_, text)| text.as_str()).collect();
                        by_line.insert(line, text.join("\n"));
                    }
                    group.clear();
                    previous_line = token.span.end_line;
                }
            }
        }
        Self { by_line, lines }
    }

    /// The documentation of the declaration that starts on the 1 based `line`.
    /// Attribute lines between the comment and the declaration are skipped.
    pub fn for_declaration(&self, line: usize) -> Option<&str> {
        let mut line = line;
        loop {
            if let Some(text) = self.by_line.get(&line) {
                return Some(text);
            }
            let above = line
                .checked_sub(2)
                .and_then(|index| self.lines.get(index))?;
            if !above.trim_start().starts_with("#[") {
                return None;
            }
            line -= 1;
        }
    }
}
