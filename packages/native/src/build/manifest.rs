use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Locate a package directory by its manifest `alias`. The directory name is
/// not required to match the alias (the bundled builtin package lives in
/// `intolang` while its alias is `builtin`).
pub fn package_dir(packages_dir: &Path, alias: &str) -> PathBuf {
    if let Ok(entries) = std::fs::read_dir(packages_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest = path.join("into-package.json");
            if let Ok(contents) = std::fs::read_to_string(&manifest)
                && let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents)
                && value.get("alias").and_then(serde_json::Value::as_str) == Some(alias)
            {
                return path;
            }
        }
    }
    packages_dir.join(alias)
}

/// The `pkg-config` libraries the package manifest declares for `source`
/// under `nativeDependencies`, keyed by the source path relative to the
/// package root.
pub fn native_dependencies(package: &Path, source: &Path) -> Result<Vec<String>> {
    let manifest = package.join("into-package.json");
    let contents = std::fs::read_to_string(&manifest).with_context(|| {
        format!(
            "Could not read native package manifest {}",
            manifest.display()
        )
    })?;
    let value: serde_json::Value = serde_json::from_str(&contents)
        .with_context(|| format!("Invalid JSON in {}", manifest.display()))?;
    let relative = source
        .strip_prefix(package)
        .with_context(|| format!("Native source {} is outside its package", source.display()))?;
    let key = relative.to_string_lossy().replace('\\', "/");
    Ok(value
        .get("nativeDependencies")
        .and_then(|deps| deps.get(key))
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect())
}
