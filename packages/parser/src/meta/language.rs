use errors::{Error, Errors, Result};
use std::fmt;
use std::str::FromStr;
use types::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageLexeme(Language);

impl FromStr for LanguageLexeme {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        match text {
            "c" => Ok(Self(Language::C)),
            "cpp" => Ok(Self(Language::Cpp)),
            other => Err(Errors::unsupported("native language", other).into()),
        }
    }
}

impl TryFrom<&str> for LanguageLexeme {
    type Error = Error;

    fn try_from(text: &str) -> Result<Self> {
        text.parse()
    }
}

impl From<Language> for LanguageLexeme {
    fn from(language: Language) -> Self {
        Self(language)
    }
}

impl From<LanguageLexeme> for Language {
    fn from(lexeme: LanguageLexeme) -> Self {
        lexeme.0
    }
}

impl fmt::Display for LanguageLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            Language::C => "c",
            Language::Cpp => "cpp",
        })
    }
}
