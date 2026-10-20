use crate::errors::Errors;

impl Errors {
    /// `text` is not the lexeme that was asked for, such as "a keyword".
    pub fn not_lexeme(text: &str, expected: &str) -> Self {
        Self::NotLexeme {
            text: text.to_string(),
            expected: expected.to_string(),
        }
    }

    /// The record of a lowered message, named by its tag or kind, has too few fields.
    pub fn malformed_record(kind: &str) -> Self {
        Self::MalformedRecord {
            kind: kind.to_string(),
        }
    }

    /// `text` names something the language does not support, such as an operator.
    pub fn unsupported(kind: &str, text: &str) -> Self {
        Self::Unsupported {
            kind: kind.to_string(),
            text: text.to_string(),
        }
    }
}
