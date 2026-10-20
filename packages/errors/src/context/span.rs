use serde::{Deserialize, Serialize};

/// A range of a source file: lines count from 1 and offsets count characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub line: usize,
    pub end_line: usize,
    pub start: usize,
    pub end: usize,
}

impl SourceSpan {
    pub fn on_line(line: usize) -> Self {
        Self {
            line,
            end_line: line,
            start: 0,
            end: 0,
        }
    }
}
