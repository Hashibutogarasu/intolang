use crate::error::Error;
use crate::errors::Errors;

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Errors::Io {
            message: error.to_string(),
        }
        .into()
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(error: std::num::ParseIntError) -> Self {
        Errors::InvalidNumber {
            reason: error.to_string(),
        }
        .into()
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Errors::Json {
            message: error.to_string(),
        }
        .into()
    }
}
