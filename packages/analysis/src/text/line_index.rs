use ipc::{Position, Range};
use types::Span;

/// Converts between the character offsets of the tokenizer and line and
/// column positions. Lines and columns count from 0; columns count characters.
pub struct LineIndex {
    chars: Vec<char>,
    starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let mut starts = vec![0];
        for (offset, character) in chars.iter().enumerate() {
            if *character == '\n' {
                starts.push(offset + 1);
            }
        }
        Self { chars, starts }
    }

    pub fn position(&self, offset: usize) -> Position {
        let offset = offset.min(self.chars.len());
        let line = self.starts.partition_point(|start| *start <= offset) - 1;
        Position::new(line as u32, (offset - self.starts[line]) as u32)
    }

    pub fn offset(&self, position: Position) -> usize {
        let line = (position.line as usize).min(self.starts.len() - 1);
        let start = self.starts[line];
        (start + position.column as usize).min(self.line_end(line))
    }

    pub fn range(&self, span: Span) -> Range {
        Range::new(self.position(span.start), self.position(span.end))
    }

    /// The text of the 1 based `line` without its leading blanks.
    pub fn line_range(&self, line: usize) -> Range {
        let index = line.saturating_sub(1).min(self.starts.len() - 1);
        let start = self.starts[index];
        let end = self.line_end(index);
        let first = (start..end)
            .find(|offset| !self.chars[*offset].is_whitespace())
            .unwrap_or(start);
        Range::new(self.position(first), self.position(end))
    }

    pub fn end(&self) -> Position {
        self.position(self.chars.len())
    }

    fn line_end(&self, line: usize) -> usize {
        let mut end = self
            .starts
            .get(line + 1)
            .map_or(self.chars.len(), |next| next - 1);
        if end > self.starts[line] && self.chars.get(end - 1) == Some(&'\r') {
            end -= 1;
        }
        end
    }
}
