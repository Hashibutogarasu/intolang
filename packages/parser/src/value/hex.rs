use anyhow::{Error, Result, bail};
use std::fmt;
use std::str::FromStr;

/// Lowercase hexadecimal text, one pair of digits per byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hex(String);

impl Hex {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Hex {
    fn from(value: &str) -> Self {
        Hex::from(value.as_bytes())
    }
}

impl From<&[u8]> for Hex {
    fn from(bytes: &[u8]) -> Self {
        let mut hex = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            hex.push_str(&format!("{byte:02x}"));
        }
        Hex(hex)
    }
}

impl FromStr for Hex {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        if text.len() % 2 != 0 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("Invalid native string argument encoding");
        }
        Ok(Hex(text.to_owned()))
    }
}

impl TryFrom<Hex> for Vec<u8> {
    type Error = Error;

    fn try_from(hex: Hex) -> Result<Self> {
        (0..hex.0.len())
            .step_by(2)
            .map(|index| Ok(u8::from_str_radix(&hex.0[index..index + 2], 16)?))
            .collect()
    }
}

impl fmt::Display for Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
