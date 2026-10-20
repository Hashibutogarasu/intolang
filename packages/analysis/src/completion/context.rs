use std::path::Path;

use ipc::{CompletionItem, CompletionKind, Position, Range};

use crate::analyzer::Analysis;

pub struct Call {
    pub callee: Vec<String>,
    pub index: usize,
}

pub enum Site {
    Nothing,
    Word { call: Option<Call> },
    Member { receiver: Vec<String> },
    Path { typed: String },
}

pub struct CompletionContext<'a> {
    pub analysis: Option<&'a Analysis>,
    pub packages_dir: Option<&'a Path>,
    pub line: usize,
    pub site: Site,
    pub prefix: String,
    pub replace: Range,
}

impl<'a> CompletionContext<'a> {
    /// The context at `position`, whose line up to the cursor is `before`.
    pub fn locate(
        before: &str,
        position: Position,
        analysis: Option<&'a Analysis>,
        packages_dir: Option<&'a Path>,
    ) -> Self {
        let chars: Vec<char> = before.chars().collect();
        let (site, prefix) = scan(&chars);
        let start = position
            .column
            .saturating_sub(prefix.chars().count() as u32);
        Self {
            analysis,
            packages_dir,
            line: position.line as usize + 1,
            site,
            prefix,
            replace: Range::new(Position::new(position.line, start), position),
        }
    }

    pub fn item(&self, label: impl Into<String>, kind: CompletionKind) -> CompletionItem {
        let label = label.into();
        CompletionItem {
            text: label.clone(),
            label,
            kind,
            detail: None,
            documentation: None,
            range: self.replace,
            snippet: false,
        }
    }
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn scan(chars: &[char]) -> (Site, String) {
    let mut string_start = None;
    let mut index = 0;
    while index < chars.len() {
        match (string_start, chars[index]) {
            (Some(_), '\\') => index += 1,
            (Some(_), '"') => string_start = None,
            (Some(_), _) => {}
            (None, '"') => string_start = Some(index),
            (None, '/') if chars.get(index + 1) == Some(&'/') => {
                return (Site::Nothing, String::new());
            }
            (None, _) => {}
        }
        index += 1;
    }
    if let Some(start) = string_start {
        return address(chars, start);
    }
    let word_start = chars
        .iter()
        .rposition(|character| !is_word(*character))
        .map_or(0, |found| found + 1);
    let prefix: String = chars[word_start..].iter().collect();
    let before = &chars[..word_start];
    if before.last() == Some(&'.') {
        return match path_before(&before[..before.len() - 1], before.len() - 1) {
            receiver if !receiver.is_empty() => (Site::Member { receiver }, prefix),
            _ => (Site::Nothing, prefix),
        };
    }
    (
        Site::Word {
            call: enclosing_call(before),
        },
        prefix,
    )
}

/// The site inside the string that opens at `start`, when it is an address.
fn address(chars: &[char], start: usize) -> (Site, String) {
    let head: String = chars[..start].iter().collect();
    if !matches!(head.trim(), "import" | "export") {
        return (Site::Nothing, String::new());
    }
    let typed: String = chars[start + 1..].iter().collect();
    let prefix = match typed.contains("://") {
        true => typed.rsplit('/').next().unwrap_or_default().to_owned(),
        false => typed.clone(),
    };
    (Site::Path { typed }, prefix)
}

/// The names of `a.b.c` that end right before `end`.
fn path_before(chars: &[char], end: usize) -> Vec<String> {
    let mut segments = Vec::new();
    let mut end = end.min(chars.len());
    loop {
        let start = chars[..end]
            .iter()
            .rposition(|character| !is_word(*character))
            .map_or(0, |found| found + 1);
        if start == end {
            return Vec::new();
        }
        segments.push(chars[start..end].iter().collect());
        if start > 0 && chars[start - 1] == '.' {
            end = start - 1;
        } else {
            segments.reverse();
            return segments;
        }
    }
}

struct Frame {
    open: char,
    commas: usize,
    callee: Vec<String>,
}

/// The call whose parentheses are still open at the end of `chars`.
fn enclosing_call(chars: &[char]) -> Option<Call> {
    let mut stack: Vec<Frame> = Vec::new();
    let mut in_string = false;
    let mut index = 0;
    while index < chars.len() {
        let character = chars[index];
        if in_string {
            match character {
                '\\' => index += 1,
                '"' => in_string = false,
                _ => {}
            }
        } else {
            match character {
                '"' => in_string = true,
                '(' => stack.push(Frame {
                    open: character,
                    commas: 0,
                    callee: path_before(chars, index),
                }),
                '[' | '{' => stack.push(Frame {
                    open: character,
                    commas: 0,
                    callee: Vec::new(),
                }),
                ')' | ']' | '}' => {
                    stack.pop();
                }
                ',' => {
                    if let Some(top) = stack.last_mut() {
                        top.commas += 1;
                    }
                }
                _ => {}
            }
        }
        index += 1;
    }
    let top = stack.pop()?;
    (top.open == '(' && !top.callee.is_empty()).then_some(Call {
        callee: top.callee,
        index: top.commas,
    })
}
