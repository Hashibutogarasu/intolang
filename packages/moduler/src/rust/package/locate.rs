use anyhow::Result;
use std::path::{Path, PathBuf};

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
