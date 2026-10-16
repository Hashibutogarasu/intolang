use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::format::Format;
use crate::syntax::support::text::strip_line_comment;

/// One source line with its comment stripped and surrounding whitespace trimmed.
#[derive(Debug, Clone)]
pub struct Line {
    pub number: usize,
    pub code: String,
}

impl Line {
    pub fn new(number: usize, raw: &str) -> Self {
        Self {
            number,
            code: strip_line_comment(raw).trim().to_owned(),
        }
    }
}

/// A read cursor over the prepared source lines.
pub struct Cursor<'a> {
    lines: &'a [Line],
    index: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(lines: &'a [Line]) -> Self {
        Self { lines, index: 0 }
    }

    pub fn at_end(&self) -> bool {
        self.index >= self.lines.len()
    }

    pub fn line(&self) -> Option<&'a Line> {
        self.lines.get(self.index)
    }

    pub fn peek(&self, offset: usize) -> Option<&'a Line> {
        self.lines.get(self.index + offset)
    }

    pub fn advance(&mut self, count: usize) {
        self.index += count;
    }
}

/// Parses one syntactic construct from the current position in the source.
///
/// Every construct also knows how to format itself, so parse and format rules
/// live together rather than in a single monolithic parser.
pub trait Parse: Format {
    /// Attempt to parse the construct starting at `cursor`.
    ///
    /// Returns `true` when the construct matched (advancing the cursor), or
    /// `false` when the line belongs to another construct. Errors carry the
    /// Into diagnostic backtrace.
    fn parse(&self, ctx: &mut ParseContext, cursor: &mut Cursor<'_>) -> Result<bool>;
}
