use errors::{Error, ErrorContext, Errors, SourceSpan, SyntaxContext};
use std::fmt;
use types::Span;

const MODULE_SCOPE: &str = "module";

/// A syntax error found while building the AST, with the line it occurred on and what the
/// parser knew about the place: the span of the token, what it expected and what it found.
/// It becomes an `errors::Error` of the Into language, which is bound to its source path with
/// `errors::Error::bound_to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    pub line: usize,
    pub scope: &'static str,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub expected: Vec<String>,
    pub found: Option<String>,
    pub hint: Option<String>,
}

impl SyntaxError {
    pub fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            scope: MODULE_SCOPE,
            message: message.into(),
            span: None,
            expected: Vec::new(),
            found: None,
            hint: None,
        }
    }

    pub fn in_scope(mut self, scope: &'static str) -> Self {
        self.scope = scope;
        self
    }

    pub fn at_span(mut self, span: Span) -> Self {
        self.span = Some(SourceSpan {
            line: span.line,
            end_line: span.end_line,
            start: span.start,
            end: span.end,
        });
        self
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

    pub fn into_error(self) -> Error {
        let kind = Errors::Syntax {
            message: self.message,
        };
        let mut syntax = SyntaxContext::in_scope(self.scope);
        syntax.expected = self.expected;
        syntax.found = self.found;
        syntax.hint = self.hint;
        let mut context = ErrorContext::of(&kind).with_syntax(syntax);
        context.span = Some(self.span.unwrap_or_else(|| SourceSpan::on_line(self.line)));
        Error::new(kind).with_context(context)
    }
}

impl From<SyntaxError> for Error {
    fn from(error: SyntaxError) -> Self {
        error.into_error()
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (line {})", self.message, self.line)
    }
}

impl std::error::Error for SyntaxError {}
