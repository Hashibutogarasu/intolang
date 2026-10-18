use anyhow::{Error, Result, bail};
use ast::UseDecl;
use std::fmt;
use std::str::FromStr;

/// The scheme that addresses the builtin package: `intolang://builtin/std`
/// names alias `builtin`, module `std`.
const BUILTIN_SCHEME: &str = "intolang";

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
            bail!("`{uri}` is not a module URI");
        };
        let (alias, module) = if scheme == BUILTIN_SCHEME {
            let mut segments = path.split('/');
            let first = segments.next().unwrap_or_default().to_owned();
            (first, segments.collect::<Vec<_>>().join("/"))
        } else {
            (scheme.to_owned(), path.to_owned())
        };
        if alias.is_empty() || module.is_empty() {
            bail!("`{uri}` does not name a module");
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
