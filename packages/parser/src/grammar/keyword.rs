use anyhow::{Error, Result, bail};
use std::fmt;
use std::str::FromStr;
use types::Keyword;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordLexeme(Keyword);

impl FromStr for KeywordLexeme {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        Ok(Self(match text {
            "init" => Keyword::Init,
            "fun" => Keyword::Fun,
            "pub" => Keyword::Pub,
            "static" => Keyword::Static,
            "macro" => Keyword::Macro,
            "into" => Keyword::Into,
            "now" => Keyword::Now,
            "late" => Keyword::Late,
            "const" => Keyword::Const,
            "unload" => Keyword::Unload,
            "enum" => Keyword::Enum,
            "if" => Keyword::If,
            "else" => Keyword::Else,
            "while" => Keyword::While,
            "break" => Keyword::Break,
            "return" => Keyword::Return,
            "throw" => Keyword::Throw,
            "import" => Keyword::Import,
            "export" => Keyword::Export,
            other => bail!("`{other}` is not a keyword"),
        }))
    }
}

impl TryFrom<&str> for KeywordLexeme {
    type Error = Error;

    fn try_from(text: &str) -> Result<Self> {
        text.parse()
    }
}

impl From<Keyword> for KeywordLexeme {
    fn from(keyword: Keyword) -> Self {
        Self(keyword)
    }
}

impl From<KeywordLexeme> for Keyword {
    fn from(lexeme: KeywordLexeme) -> Self {
        lexeme.0
    }
}

impl fmt::Display for KeywordLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            Keyword::Init => "init",
            Keyword::Fun => "fun",
            Keyword::Pub => "pub",
            Keyword::Static => "static",
            Keyword::Macro => "macro",
            Keyword::Into => "into",
            Keyword::Now => "now",
            Keyword::Late => "late",
            Keyword::Const => "const",
            Keyword::Unload => "unload",
            Keyword::Enum => "enum",
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::While => "while",
            Keyword::Break => "break",
            Keyword::Return => "return",
            Keyword::Throw => "throw",
            Keyword::Import => "import",
            Keyword::Export => "export",
        })
    }
}
