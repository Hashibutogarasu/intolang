use serde::{Deserialize, Serialize};

use super::{SourceSpan, SyntaxContext};
use crate::errors::{Domain, Errors};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

/// A place related to an error, such as the declaration it conflicts with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelatedContext {
    pub message: String,
    pub path: Option<String>,
    pub span: Option<SourceSpan>,
}

/// The context of an error as plain data, so that it can be written as JSON for tools
/// such as a language server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorContext {
    pub code: Option<u16>,
    pub domain: Domain,
    pub severity: Severity,
    pub message: String,
    pub path: Option<String>,
    pub span: Option<SourceSpan>,
    pub syntax: Option<SyntaxContext>,
    pub related: Vec<RelatedContext>,
}

impl ErrorContext {
    pub fn of(kind: &Errors) -> Self {
        Self {
            code: kind.code(),
            domain: kind.domain(),
            severity: kind.severity(),
            message: kind.to_string(),
            path: None,
            span: None,
            syntax: None,
            related: Vec::new(),
        }
    }

    pub fn at(mut self, path: impl Into<String>, span: SourceSpan) -> Self {
        self.path = Some(path.into());
        self.span = Some(span);
        self
    }

    pub fn with_syntax(mut self, syntax: SyntaxContext) -> Self {
        self.syntax = Some(syntax);
        self
    }

    pub fn relate(mut self, related: RelatedContext) -> Self {
        self.related.push(related);
        self
    }
}
