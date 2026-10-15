use anyhow::{Result, anyhow, bail};
use std::{collections::HashSet, path::Path};

/// Resolve Into package and native-language module imports using package exports.
pub(super) fn resolve(source: &str, packages_dir: &Path) -> Result<HashSet<String>> {
    let mut imported_functions = HashSet::new();
    for line in source.lines() {
        let line = line.trim();
        let Some(uri) = line.strip_prefix("import ").and_then(|v| v.strip_prefix('"'))
            .and_then(|v| v.split_once('"').map(|(uri, _)| uri)) else { continue; };
        let Some((scheme, path)) = uri.split_once("://") else {
            bail!("Invalid import URI: {uri}");
        };
        let (alias, module) = match scheme {
            "intolang" => {
                let mut segments = path.split('/');
                (segments.next().unwrap_or_default(), segments.collect::<Vec<_>>().join("/"))
            }
            "c" | "cpp" => (scheme, path.to_owned()),
            _ => continue,
        };
        if alias.is_empty() || module.is_empty() { bail!("Invalid Into package import: {uri}"); }

        let package = find_by_alias(packages_dir, alias)?
            .ok_or_else(|| anyhow!("Imported package or language module `{alias}` is not bundled"))?;
        let info_path = package.join("src/main/into-info.into");
        let info = std::fs::read_to_string(&info_path)
            .map_err(|_| anyhow!("Package `{alias}` has no into-info.into"))?;
        let expected = format!("{alias}/{module}/");
        let exact_export = format!("{alias}/{module}.;");
        let exports = info.lines().filter_map(|decl| {
            let export = decl.trim().strip_prefix("pub ")?;
            let function = if export == exact_export {
                module.rsplit('/').next().unwrap_or(module.as_str())
            } else {
                export.strip_prefix(&expected)?.trim_end_matches(';').trim_end_matches('.')
            };
            (!function.is_empty()).then(|| function.to_owned())
        }).collect::<Vec<_>>();
        if exports.is_empty() {
            bail!("Package `{alias}` does not export module `{module}`");
        }
        imported_functions.extend(exports);
    }
    Ok(imported_functions)
}

pub(super) fn find_by_alias(packages_dir: &Path, alias: &str) -> Result<Option<std::path::PathBuf>> {
    for entry in std::fs::read_dir(packages_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() { continue; }
        let manifest_path = entry.path().join("into-package.json");
        let Ok(contents) = std::fs::read_to_string(manifest_path) else { continue; };
        let Ok(manifest): std::result::Result<serde_json::Value, _> = serde_json::from_str(&contents) else { continue; };
        if manifest.get("alias").and_then(serde_json::Value::as_str) == Some(alias) {
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}
