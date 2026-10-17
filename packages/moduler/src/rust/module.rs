use std::path::PathBuf;

use super::package::PackageInfo;
use super::text::strip_keyword;

/// A parsed `import "scheme://module";` URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSpec {
    pub alias: String,
    pub module: String,
}

impl ImportSpec {
    /// The canonical `alias://module` spelling.
    pub fn uri(&self) -> String {
        format!("{}://{}", self.alias, self.module)
    }
}

/// A parsed `export "scheme://path";` URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSpec {
    pub alias: String,
    pub path: String,
}

/// A module reached from an entry source, directly or transitively.
///
/// Mirrors the Into `Module` class: it holds the [`PackageInfo`] it belongs to
/// and the path of the module source file (`sourcePath: Path`).
#[derive(Debug, Clone)]
pub struct Module {
    pub package: PackageInfo,
    pub source_path: PathBuf,
    pub alias: String,
    pub name: String,
}

/// Parse an `import "scheme://module";` line, resolving `intolang://` to the
/// builtin package. Returns `None` when the line is not an import.
///
/// The `import` keyword is matched as a word so the amount of whitespace before
/// the URI does not matter.
pub fn parse_import(line: &str) -> Option<ImportSpec> {
    let rest = strip_keyword(line.trim(), "import")?;
    let rest = rest.trim_start().strip_prefix('"')?;
    let uri = rest.split_once('"')?.0;
    let (scheme, path) = uri.split_once("://")?;
    let (alias, module) = match scheme {
        "intolang" => {
            let mut segments = path.split('/');
            let first = segments.next().unwrap_or_default().to_owned();
            (first, segments.collect::<Vec<_>>().join("/"))
        }
        _ => (scheme.to_owned(), path.to_owned()),
    };
    if alias.is_empty() || module.is_empty() {
        return None;
    }
    Some(ImportSpec { alias, module })
}

/// Read an `export "alias://path";` declaration. Returns `None` when the line
/// is not an export.
///
/// The `export` keyword is matched as a word so the amount of whitespace before
/// the URI does not matter.
pub fn parse_export(line: &str) -> Option<ExportSpec> {
    let rest = strip_keyword(line.trim(), "export")?;
    let uri = rest.trim_start().strip_prefix('"')?.split_once('"')?.0;
    let (alias, path) = uri.split_once("://")?;
    (!alias.is_empty() && !path.is_empty()).then(|| ExportSpec {
        alias: alias.to_owned(),
        path: path.to_owned(),
    })
}
