use std::collections::HashMap;

use ipc::{Position, uri_to_path};

/// The text of the open documents, which the editor owns, and of other files
/// the answers point into, which are read when asked.
#[derive(Default)]
pub struct Documents {
    open: HashMap<String, String>,
}

impl Documents {
    pub fn set(&mut self, uri: &str, text: String) {
        self.open.insert(uri.to_owned(), text);
    }

    pub fn remove(&mut self, uri: &str) {
        self.open.remove(uri);
    }

    pub fn text(&self, uri: &str) -> String {
        self.open
            .get(uri)
            .cloned()
            .or_else(|| std::fs::read_to_string(uri_to_path(uri)?).ok())
            .unwrap_or_default()
    }

    pub fn analyzer_position(&self, uri: &str, position: lsp_types::Position) -> Position {
        Position::new(
            position.line,
            super::from_lsp(&self.text(uri), position.line, position.character),
        )
    }

    pub fn range(&self, uri: &str, range: ipc::Range) -> lsp_types::Range {
        let text = self.text(uri);
        let convert = |position: Position| {
            lsp_types::Position::new(
                position.line,
                super::to_lsp(&text, position.line, position.column),
            )
        };
        lsp_types::Range::new(convert(range.start), convert(range.end))
    }
}
