use anyhow::{Error, Result, bail};
use std::fmt;
use std::str::FromStr;

/// A path inside a bundle: relative, normalized and separated by `/`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BundlePath(String);

impl BundlePath {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }

    pub fn file_name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or_default()
    }

    /// The directory that holds the path; `None` for a top level path.
    pub fn parent(&self) -> Option<BundlePath> {
        self.0
            .rsplit_once('/')
            .map(|(parent, _)| BundlePath(parent.to_owned()))
    }
}

impl TryFrom<&str> for BundlePath {
    type Error = Error;

    fn try_from(text: &str) -> Result<Self> {
        if text.is_empty() {
            bail!("A bundle path cannot be empty");
        }
        if text.starts_with('/') {
            bail!("`{text}` is absolute; a bundle path is relative");
        }
        if text.contains('\\') || text.contains('\0') {
            bail!("`{text}` holds a character a bundle path cannot hold");
        }
        for component in text.split('/') {
            match component {
                "" => bail!("`{text}` has an empty component"),
                "." | ".." => bail!("`{text}` has the component `{component}`"),
                _ => {}
            }
        }
        Ok(Self(text.to_owned()))
    }
}

impl FromStr for BundlePath {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        Self::try_from(text)
    }
}

impl fmt::Display for BundlePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
