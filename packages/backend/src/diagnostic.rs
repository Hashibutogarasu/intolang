use anyhow::{Error, anyhow};
use ast::SyntaxError;

/// An error in the Into language backtrace format.
pub fn diagnostic(path: &str, line: usize, scope: &str, message: &str) -> Error {
    anyhow!("{message}\n\nInto language backtrace:\n  0: {scope} at {path}:{line}")
}

/// Bind a parse failure to the source path it occurred in; failures that are
/// not syntax errors (such as an unterminated string) pass through.
pub fn syntax_diagnostic(path: &str, error: Error) -> Error {
    match error.downcast_ref::<SyntaxError>() {
        Some(syntax) => anyhow!("{}", syntax.backtrace(path)),
        None => error,
    }
}
