use errors::{Error, Errors, Result};
use parser::Hex;
use std::fmt;
use std::str::FromStr;

/// An argument of a native call as carried in a lowered message: `s:<hex>`
/// for a string, `i:<n>` for a 32-bit integer, `p:<slot>` for the value
/// held in a named slot and `f:<slot>:<class>:<index>` for a field of the
/// instance a slot holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeArg {
    Str(Vec<u8>),
    Int(i32),
    Slot(String),
    Field {
        slot: String,
        class: String,
        index: usize,
    },
}

impl From<&str> for NativeArg {
    fn from(value: &str) -> Self {
        NativeArg::Str(value.as_bytes().to_vec())
    }
}

impl From<i32> for NativeArg {
    fn from(value: i32) -> Self {
        NativeArg::Int(value)
    }
}

impl FromStr for NativeArg {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        let (kind, value) = text.split_once(':').unwrap_or(("", ""));
        match kind {
            "s" => Ok(NativeArg::Str(value.parse::<Hex>()?.try_into()?)),
            "i" => Ok(NativeArg::Int(value.parse()?)),
            "p" => Ok(NativeArg::Slot(value.to_owned())),
            "f" => {
                let mut parts = value.splitn(3, ':');
                match (parts.next(), parts.next(), parts.next()) {
                    (Some(slot), Some(class), Some(index)) => Ok(NativeArg::Field {
                        slot: slot.to_owned(),
                        class: class.to_owned(),
                        index: index.parse()?,
                    }),
                    _ => Err(Errors::MalformedFieldArgument {
                        text: text.to_string(),
                    }
                    .into()),
                }
            }
            other => Err(Errors::UnsupportedLinkArgument {
                kind: other.to_string(),
            }
            .into()),
        }
    }
}

impl fmt::Display for NativeArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NativeArg::Str(bytes) => write!(f, "s:{}", Hex::from(bytes.as_slice())),
            NativeArg::Int(value) => write!(f, "i:{value}"),
            NativeArg::Slot(name) => write!(f, "p:{name}"),
            NativeArg::Field { slot, class, index } => write!(f, "f:{slot}:{class}:{index}"),
        }
    }
}
