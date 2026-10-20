use errors::{Error, ErrorContext, Errors, IntoLangStackTrace, SourceSpan, SyntaxContext};
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

    /// The error as an `errors::Error` of the Into language bound to `path`.
    pub fn into_error(&self, path: &str) -> Error {
        let kind = Errors::Syntax {
            message: self.message.clone(),
        };
        let context = ErrorContext::of(&kind)
            .at(path, SourceSpan::on_line(self.line))
            .with_syntax(SyntaxContext::in_scope(self.scope));
        Error::into_lang(kind, IntoLangStackTrace::at(self.scope, path, self.line))
            .with_context(context)
    }

    /// Bind a parse failure to the source path it occurred in; failures that are
    /// not syntax errors (such as an unterminated string) pass through.
    pub fn bind(error: anyhow::Error, path: &str) -> anyhow::Error {
        match error.downcast_ref::<Self>() {
            Some(syntax) => syntax.into_error(path).into(),
            None => error,
        }
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (line {})", self.message, self.line)
    }
}

impl std::error::Error for SyntaxError {}
