use serde::{Deserialize, Serialize};

use crate::catalog::CatalogEntry;

/// Which side raised an error: the Into program or the toolchain itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    IntoLang,
    Compiler,
}

macro_rules! declare_errors {
    (
        $(
            $domain:ident {
                $(
                    $(#[$meta:meta])*
                    $name:ident $({ $($field:ident : $fty:ty),+ $(,)? })? = $code:literal,
                )*
            }
        )*
    ) => {
        /// Every error of the toolchain with its code, message and domain.
        #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
        #[repr(u16)]
        pub enum Errors {
            $($(
                $(#[$meta])*
                $name $({ $($field : $fty),+ })? = $code,
            )*)*
            #[error("Unknown error")]
            Unknown = 0xFFFF,
        }

        impl Errors {
            /// The code of the error; `Unknown` has none.
            pub const fn code(&self) -> Option<u16> {
                match self {
                    $($(Self::$name { .. } => Some($code),)*)*
                    Self::Unknown => None,
                }
            }

            pub const fn domain(&self) -> Domain {
                match self {
                    $($(Self::$name { .. } => Domain::$domain,)*)*
                    Self::Unknown => Domain::Compiler,
                }
            }
        }

        pub(crate) const ENTRIES: &[CatalogEntry] = &[
            $($(
                CatalogEntry {
                    code: $code,
                    domain: Domain::$domain,
                    name: stringify!($name),
                },
            )*)*
        ];
    };
}

declare_errors! {
    IntoLang {
        #[error("{message}")]
        Syntax { message: String } = 0x0001,
        #[error("`{text}` is not {expected}")]
        NotLexeme { text: String, expected: String } = 0x0002,
        #[error("Unsupported {kind} `{text}`")]
        Unsupported { kind: String, text: String } = 0x0003,
        #[error("Expected a type name")]
        ExpectedTypeName = 0x0004,
        #[error("{construct} needs {needs}")]
        TypeNeeds { construct: String, needs: String } = 0x0005,
        #[error("Type arguments need a type name before `<`")]
        TypeArgumentsNeedName = 0x0006,
        #[error("A character literal holds exactly one character: '{raw}'")]
        InvalidCharLiteral { raw: String } = 0x0007,
        #[error("Unterminated string at line {line}")]
        UnterminatedString { line: usize } = 0x0008,
        #[error("Unterminated character literal at line {line}")]
        UnterminatedChar { line: usize } = 0x0009,
        #[error("{message}")]
        Semantic { message: String } = 0x1001,
        #[error("{message}")]
        Ownership { message: String } = 0x2001,
        #[error("{message}")]
        Runtime { message: String } = 0x6001,
    }
    Compiler {
        #[error("{message}")]
        Package { message: String } = 0x8001,
        #[error("{message}")]
        Native { message: String } = 0x9001,
        #[error("{message}")]
        Codegen { message: String } = 0xA001,
        #[error("{message}")]
        Io { message: String } = 0xB001,
        #[error("{message}")]
        Internal { message: String } = 0xC001,
        #[error("{message}")]
        Panic { message: String } = 0xC002,
        #[error("Unknown message tag `{tag}`")]
        UnknownMessageTag { tag: String } = 0xC003,
        #[error("Not a directive line")]
        NotDirectiveLine = 0xC004,
        #[error("Invalid native string argument encoding")]
        InvalidHexEncoding = 0xC005,
    }
}
