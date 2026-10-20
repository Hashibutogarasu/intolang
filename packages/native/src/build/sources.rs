use errors::{Errors, Result};
use parser::Record;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use types::{Language, MessageTag};

use super::manifest::package_dir;
use crate::call::LinkCall;

/// A foreign source file a lowered message links against.
pub struct NativeSource {
    pub path: PathBuf,
    pub package: PathBuf,
    /// The alias of the package that owns the source.
    pub alias: String,
    /// The path of the source relative to its package.
    pub relative: String,
    pub language: Language,
}

/// The distinct native sources the `NATIVE_LINK_ASSIGN` records of `message`
/// refer to, in order of first use.
pub fn collect_sources(message: &str, packages_dir: &Path) -> Result<Vec<NativeSource>> {
    let mut seen = HashSet::new();
    let mut sources = Vec::new();
    for line in message.lines() {
        let Ok(record) = line.parse::<Record>() else {
            continue;
        };
        if record.tag() != MessageTag::LinkAssign {
            continue;
        }
        let call = LinkCall::try_from(&record)?;
        let package = package_dir(packages_dir, &call.package_alias);
        let path = package.join(&call.source_path);
        if !path.is_file() {
            return Err(Errors::NativeSourceMissing {
                path: path.display().to_string(),
            }
            .into());
        }
        if seen.insert(path.clone()) {
            sources.push(NativeSource {
                path,
                package,
                alias: call.package_alias,
                relative: call.source_path,
                language: call.language,
            });
        }
    }
    Ok(sources)
}
