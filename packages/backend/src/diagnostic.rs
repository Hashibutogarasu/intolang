use anyhow::Error;
use errors::{Errors, IntoLangStackTrace};

/// An error of the Into program bound to the source path, the line and the scope it was found in.
pub fn diagnostic(path: &str, line: usize, scope: &str, message: &str) -> Error {
    let kind = Errors::Semantic {
        message: message.to_string(),
    };
    errors::Error::into_lang(kind, IntoLangStackTrace::at(scope, path, line)).into()
}
