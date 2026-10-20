use ipc::{Position, Range};

/// Where the C or C++ source defines `symbol`: its name at the start of a
/// function definition, or at its declaration when the source has no
/// definition of it.
pub fn locate_symbol(text: &str, symbol: &str) -> Option<Range> {
    let starts = line_starts(text);
    let mut declaration = None;
    for (offset, _) in text.match_indices(symbol) {
        let end = offset + symbol.len();
        if !is_word_boundary(text, offset, end) {
            continue;
        }
        let Some(open) = opening_parenthesis(text, end) else {
            continue;
        };
        let line = starts.partition_point(|start| *start <= offset) - 1;
        let head = &text[starts[line]..offset];
        if !has_return_type(text, &starts, line, head) {
            continue;
        }
        let Some(close) = closing_parenthesis(text, open) else {
            continue;
        };
        let range = Range::new(
            Position::new(line as u32, head.chars().count() as u32),
            Position::new(
                line as u32,
                (head.chars().count() + symbol.chars().count()) as u32,
            ),
        );
        match text[close + 1..].trim_start().chars().next() {
            Some('{') => return Some(range),
            Some(';') => {
                declaration.get_or_insert(range);
            }
            _ => {}
        }
    }
    declaration
}

fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(newline, _)| newline + 1))
        .collect()
}

fn is_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    !text[..start].chars().next_back().is_some_and(word)
        && !text[end..].chars().next().is_some_and(word)
}

fn opening_parenthesis(text: &str, name_end: usize) -> Option<usize> {
    let rest = &text[name_end..];
    let trimmed = rest.trim_start();
    trimmed
        .starts_with('(')
        .then(|| name_end + rest.len() - trimmed.len())
}

fn closing_parenthesis(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, c) in text[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Whether a type stands before the name, on its line or on the line above,
/// which tells a definition or declaration from a call or a comment.
fn has_return_type(text: &str, starts: &[usize], line: usize, head: &str) -> bool {
    let first = head.trim_start();
    if ["//", "*", "/*", "#"]
        .iter()
        .any(|comment| first.starts_with(comment))
    {
        return false;
    }
    let before = match head.trim_end() {
        "" => (0..line)
            .rev()
            .map(|above| {
                let end = starts.get(above + 1).copied().unwrap_or(text.len());
                text[starts[above]..end].trim_end()
            })
            .find(|above| !above.is_empty())
            .unwrap_or_default(),
        before => before,
    };
    !before.ends_with("return")
        && before
            .chars()
            .next_back()
            .is_some_and(|last| last.is_alphanumeric() || matches!(last, '_' | '*' | '&'))
}
