use ast::UseDecl;
use errors::{Error, Errors, Result};
use std::fmt;
use std::str::FromStr;

/// A parsed `export "alias://path";` URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSpec {
    pub alias: String,
    pub path: String,
}

impl FromStr for ExportSpec {
    type Err = Error;

    fn from_str(uri: &str) -> Result<Self> {
        let Some((alias, path)) = uri.split_once("://") else {
            return Err(Errors::NotModuleUri {
                uri: uri.to_string(),
            }
            .into());
        };
        if alias.is_empty() || path.is_empty() {
            return Err(Errors::ExportNameMissing {
                uri: uri.to_string(),
            }
            .into());
        }
        Ok(Self {
            alias: alias.to_owned(),
            path: path.to_owned(),
        })
    }
}

impl TryFrom<&UseDecl> for ExportSpec {
    type Error = Error;

    fn try_from(decl: &UseDecl) -> Result<Self> {
        decl.uri.parse()
    }
}

impl fmt::Display for ExportSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", self.alias, self.path)
    }
}
