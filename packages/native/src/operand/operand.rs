use errors::{Error, Result};
use parser::Hex;
use std::fmt;
use std::str::FromStr;

/// A value operand of a lowered record: a numeric constant (`n:5`), the value
/// in a named slot (`p:count`) or a string (`s:<hex>`). An operand without a
/// kind is read as a numeric constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Number(String),
    Slot(String),
    Str(Vec<u8>),
}

impl FromStr for Operand {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        match text.split_once(':') {
            Some(("p", name)) => Ok(Operand::Slot(name.to_owned())),
            Some(("s", hex)) => Ok(Operand::Str(hex.parse::<Hex>()?.try_into()?)),
            Some((_, value)) => Ok(Operand::Number(value.to_owned())),
            None => Ok(Operand::Number(text.to_owned())),
        }
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Number(value) => write!(f, "n:{value}"),
            Operand::Slot(name) => write!(f, "p:{name}"),
            Operand::Str(bytes) => write!(f, "s:{}", Hex::from(bytes.as_slice())),
        }
    }
}
