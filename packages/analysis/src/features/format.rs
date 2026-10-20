use ipc::{Position, Range, TextEdit};

use crate::text::LineIndex;

/// One edit that replaces the whole text, or none when it is formatted already.
pub fn format(text: &str, line_index: &LineIndex) -> Vec<TextEdit> {
    let formatted = formatter::format_source(text);
    if formatted == text {
        return Vec::new();
    }
    vec![TextEdit {
        range: Range::new(Position::new(0, 0), line_index.end()),
        text: formatted,
    }]
}
