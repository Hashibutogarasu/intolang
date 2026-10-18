use std::convert::Infallible;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// A filesystem path as it appears in messages and diagnostics: lossy UTF-8
/// text with the platform's separators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathLexeme(String);

impl PathLexeme {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&Path> for PathLexeme {
    fn from(path: &Path) -> Self {
        Self(path.to_string_lossy().into_owned())
    }
}

impl From<&PathBuf> for PathLexeme {
    fn from(path: &PathBuf) -> Self {
        Self::from(path.as_path())
    }
}

impl FromStr for PathLexeme {
    type Err = Infallible;

    fn from_str(text: &str) -> Result<Self, Infallible> {
        Ok(Self(text.to_owned()))
    }
}

impl From<PathLexeme> for PathBuf {
    fn from(lexeme: PathLexeme) -> Self {
        PathBuf::from(lexeme.0)
    }
}

impl From<PathLexeme> for String {
    fn from(lexeme: PathLexeme) -> Self {
        lexeme.0
    }
}

impl fmt::Display for PathLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
