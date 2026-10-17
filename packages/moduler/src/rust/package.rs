use anyhow::Result;
use std::path::{Path, PathBuf};

/// Metadata describing an Into package, mirroring the `PackageInfo` class.
#[derive(Debug, Clone, Default)]
pub struct PackageInfo {
    pub alias: String,
    pub name: String,
    pub version: String,
    pub licence: String,
    pub root: PathBuf,
}

impl PackageInfo {
    pub fn new(alias: &str, root: PathBuf, manifest: &serde_json::Value) -> Self {
        let text = |key: &str| {
            manifest
                .get(key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        Self {
            alias: alias.to_owned(),
            name: text("name"),
            version: text("version"),
            licence: text("LICENCE"),
            root,
        }
    }
}

/// Locate a package directory by its manifest `alias`. The directory name is
/// not required to match the alias.
pub fn find_by_alias(packages_dir: &Path, alias: &str) -> Result<Option<PathBuf>> {
    for entry in std::fs::read_dir(packages_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(entry.path().join("into-package.json")) else {
            continue;
        };
        let Ok(manifest): std::result::Result<serde_json::Value, _> =
            serde_json::from_str(&contents)
        else {
            continue;
        };
        if manifest.get("alias").and_then(serde_json::Value::as_str) == Some(alias) {
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}
