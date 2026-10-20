use std::fmt::Display;
use std::path::Path;

use crate::errors::Errors;

impl Errors {
    pub fn read(path: &Path, reason: impl Display) -> Self {
        Self::Read {
            path: path.display().to_string(),
            reason: reason.to_string(),
        }
    }

    pub fn write(path: &Path, reason: impl Display) -> Self {
        Self::Write {
            path: path.display().to_string(),
            reason: reason.to_string(),
        }
    }

    /// A failed action on a file, such as "link", "remove" or "update".
    pub fn file_action(action: &str, path: &Path, reason: impl Display) -> Self {
        Self::FileAction {
            action: action.to_string(),
            path: path.display().to_string(),
            reason: reason.to_string(),
        }
    }
}
