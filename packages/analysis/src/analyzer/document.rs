use super::analysis::Analysis;
use crate::text::LineIndex;

pub struct Document {
    pub text: String,
    pub version: i64,
    pub line_index: LineIndex,
    pub analysis: Option<Analysis>,
}

impl Document {
    pub fn new(text: String, version: i64) -> Self {
        Self {
            line_index: LineIndex::new(&text),
            text,
            version,
            analysis: None,
        }
    }

    pub fn replace(&mut self, text: String, version: i64) {
        self.line_index = LineIndex::new(&text);
        self.text = text;
        self.version = version;
    }
}
