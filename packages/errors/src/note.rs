use crate::error::{Error, Result};
use crate::errors::Errors;

/// Say what was being done when a result failed, without changing the kind of its error.
pub trait Context<T> {
    fn context(self, note: impl Into<String>) -> Result<T>;

    fn with_context<N: Into<String>>(self, note: impl FnOnce() -> N) -> Result<T>;
}

impl<T> Context<T> for Result<T> {
    fn context(self, note: impl Into<String>) -> Result<T> {
        self.map_err(|error| error.with_note(note))
    }

    fn with_context<N: Into<String>>(self, note: impl FnOnce() -> N) -> Result<T> {
        self.map_err(|error| error.with_note(note()))
    }
}

/// A missing value is an error of the toolchain that the note describes.
impl<T> Context<T> for Option<T> {
    fn context(self, note: impl Into<String>) -> Result<T> {
        self.ok_or_else(|| {
            Error::from(Errors::Internal {
                message: note.into(),
            })
        })
    }

    fn with_context<N: Into<String>>(self, note: impl FnOnce() -> N) -> Result<T> {
        self.ok_or_else(|| {
            Error::from(Errors::Internal {
                message: note().into(),
            })
        })
    }
}
