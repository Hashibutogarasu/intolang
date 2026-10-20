use errors::{Error, IntoLangStackTrace};

/// An error of the Into program bound to the source path, the line and the scope it was found in.
pub fn diagnostic(path: &str, line: usize, scope: &str, error: Error) -> Error {
    error.with_into(IntoLangStackTrace::at(scope, path, line))
}
