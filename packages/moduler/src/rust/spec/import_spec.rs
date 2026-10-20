use ast::UseDecl;
use errors::{Error, Errors, Result};
use std::fmt;
use std::str::FromStr;

use crate::package::BUILTIN_SCHEME;

/// A parsed `import "scheme://module";` URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSpec {
    pub alias: String,
    pub module: String,
}

impl FromStr for ImportSpec {
    type Err = Error;

    fn from_str(uri: &str) -> Result<Self> {
        let Some((scheme, path)) = uri.split_once("://") else {
            return Err(Errors::NotModuleUri {
                uri: uri.to_string(),
            }
            .into());
        };
        let (alias, module) = if scheme == BUILTIN_SCHEME {
            let mut segments = path.split('/');
            let first = segments.next().unwrap_or_default().to_owned();
            (first, segments.collect::<Vec<_>>().join("/"))
        } else {
            (scheme.to_owned(), path.to_owned())
        };
        if alias.is_empty() || module.is_empty() {
            return Err(Errors::ImportNameMissing {
                uri: uri.to_string(),
            }
            .into());
        }
        Ok(Self { alias, module })
    }
}

impl TryFrom<&UseDecl> for ImportSpec {
    type Error = Error;

    fn try_from(decl: &UseDecl) -> Result<Self> {
        decl.uri.parse()
    }
}

/// The canonical `alias://module` spelling.
impl fmt::Display for ImportSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", self.alias, self.module)
    }
}
