use errors::{Error, ErrorContext, Errors, IntoLangStackTrace, SourceSpan, SyntaxContext};
use std::fmt;
use types::Span;

const MODULE_SCOPE: &str = "module";

/// A syntax error found while building the AST, with the line it occurred on and what the
/// parser knew about the place: the span of the token, what it expected and what it found.
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

    /// The error as an `errors::Error` of the Into language bound to `path`.
    pub fn into_error(&self, path: &str) -> Error {
        let kind = Errors::Syntax {
            message: self.message.clone(),
        };
        let mut syntax = SyntaxContext::in_scope(self.scope);
        syntax.expected = self.expected.clone();
        syntax.found = self.found.clone();
        syntax.hint = self.hint.clone();
        let span = self.span.unwrap_or_else(|| SourceSpan::on_line(self.line));
        let context = ErrorContext::of(&kind).at(path, span).with_syntax(syntax);
        Error::into_lang(kind, IntoLangStackTrace::at(self.scope, path, self.line))
            .with_context(context)
    }

    /// Bind a parse failure to the source path it occurred in. Failures of the
    /// tokenizer that know their line are bound too; others pass through.
    pub fn bind(error: anyhow::Error, path: &str) -> anyhow::Error {
        if let Some(syntax) = error.downcast_ref::<Self>() {
            return syntax.into_error(path).into();
        }
        if let Some(found) = error.downcast_ref::<Error>() {
            let line = match found.kind() {
                Errors::UnterminatedString { line } | Errors::UnterminatedChar { line } => *line,
                _ => return error,
            };
            let kind = found.kind().clone();
            let context = ErrorContext::of(&kind).at(path, SourceSpan::on_line(line));
            return Error::into_lang(kind, IntoLangStackTrace::at(MODULE_SCOPE, path, line))
                .with_context(context)
                .into();
        }
        error
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (line {})", self.message, self.line)
    }
}

impl std::error::Error for SyntaxError {}
