use std::fmt;

/// A syntax error found while building the AST, with the line it occurred on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    pub line: usize,
    pub scope: &'static str,
    pub message: String,
}

impl SyntaxError {
    pub fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            scope: "module",
            message: message.into(),
        }
    }

    pub fn in_scope(mut self, scope: &'static str) -> Self {
        self.scope = scope;
        self
    }

    /// The error in the Into language backtrace format for `path`.
    pub fn backtrace(&self, path: &str) -> String {
        format!(
            "{}\n\nInto language backtrace:\n  0: {} at {path}:{}",
            self.message, self.scope, self.line
        )
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (line {})", self.message, self.line)
    }
}

impl std::error::Error for SyntaxError {}
