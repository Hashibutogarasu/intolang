use serde::{Deserialize, Serialize};

/// What a syntax error knows about the place it was found in.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxContext {
    pub scope: String,
    pub expected: Vec<String>,
    pub found: Option<String>,
    pub hint: Option<String>,
}

impl SyntaxContext {
    pub fn in_scope(scope: impl Into<String>) -> Self {
        Self {
            scope: scope.into(),
            ..Self::default()
        }
    }

    pub fn expecting(mut self, expected: impl Into<String>) -> Self {
        self.expected.push(expected.into());
        self
    }

    pub fn found(mut self, found: impl Into<String>) -> Self {
        self.found = Some(found.into());
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}
