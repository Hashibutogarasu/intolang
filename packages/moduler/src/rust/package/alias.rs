use errors::{Error, Errors, Result};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackageAlias(String);

impl PackageAlias {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for PackageAlias {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        let valid = !text.is_empty()
            && text != "."
            && text != ".."
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
        if !valid {
            return Err(Errors::PackageAliasInvalid {
                text: text.to_string(),
            }
            .into());
        }
        Ok(Self(text.to_owned()))
    }
}

impl TryFrom<&str> for PackageAlias {
    type Error = Error;

    fn try_from(text: &str) -> Result<Self> {
        text.parse()
    }
}

impl fmt::Display for PackageAlias {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
