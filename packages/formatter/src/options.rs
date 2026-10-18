/// One level of block indentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indent {
    Spaces(usize),
    Tab,
}

impl Indent {
    pub(crate) fn unit(self) -> String {
        match self {
            Indent::Spaces(width) => " ".repeat(width),
            Indent::Tab => "\t".to_owned(),
        }
    }
}

/// Layout choices the formatter applies; `Default` is the canonical style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatOptions {
    pub indent: Indent,
    /// Longest run of blank lines kept between statements.
    pub max_blank_lines: usize,
    /// Extra indent levels for lines continued inside parentheses.
    pub continuation_levels: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: Indent::Spaces(4),
            max_blank_lines: 1,
            continuation_levels: 1,
        }
    }
}
