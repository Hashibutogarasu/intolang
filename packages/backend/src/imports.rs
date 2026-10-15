use anyhow::{Result, anyhow, bail};
use std::{collections::HashSet, path::Path};

#[derive(Debug, Clone)]
pub(super) struct NativeLink {
    pub language: String,
    pub package_alias: String,
    pub source_path: String,
    pub target: String,
    pub return_type: String,
}

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

/// Read `#[link("c|cpp", path, symbol)]` metadata attached to imported
/// foreign declarations. `path` is relative to the imported package root.
pub(super) fn resolve_links(source: &str, packages_dir: &Path) -> Result<std::collections::HashMap<String, NativeLink>> {
    let mut links = std::collections::HashMap::new();
    for line in source.lines() {
        let line = line.trim();
        let Some(uri) = line.strip_prefix("import ").and_then(|v| v.strip_prefix('"'))
            .and_then(|v| v.split_once('"').map(|(uri, _)| uri)) else { continue; };
        let Some((alias, module)) = uri.split_once("://") else { continue; };
        if !matches!(alias, "c" | "cpp") { continue; }
        let Some(package) = find_by_alias(packages_dir, alias)? else { continue; };
        let declaration = package.join("src/main").join(format!("{module}.into"));
        let Ok(contents) = std::fs::read_to_string(declaration) else { continue; };
        let mut pending: Option<NativeLink> = None;
        for declaration in contents.lines().map(str::trim) {
            if declaration.starts_with("#[link(") {
                pending = parse_link_attribute(declaration, alias, &package)?;
                continue;
            }
            if let Some(link) = pending.take() {
                if let Some(rest) = declaration.split_once("fun ").map(|(_, rest)| rest) {
                    if let Some((name, _)) = rest.split_once('(') {
                        let return_type = rest.rsplit_once("):").map(|(_, ty)| ty.trim().trim_end_matches(';').trim().to_owned()).unwrap_or_else(|| "Void".into());
                        let mut link = link;
                        link.return_type = return_type;
                        links.insert(name.trim().to_owned(), link);
                    }
                }
            }
        }
    }
    Ok(links)
}

pub(super) fn imported_module_sources(source: &str, packages_dir: &Path) -> Result<Vec<String>> {
    let mut modules = Vec::new();
    for line in source.lines().map(str::trim) {
        let Some(uri) = line.strip_prefix("import ").and_then(|v| v.strip_prefix('"'))
            .and_then(|v| v.split_once('"').map(|(uri, _)| uri)) else { continue; };
        let Some((alias, module)) = uri.split_once("://") else { continue; };
        let Some(package) = find_by_alias(packages_dir, alias)? else { continue; };
        let module_path = package.join("src/main").join(format!("{module}.into"));
        if let Ok(contents) = std::fs::read_to_string(module_path) { modules.push(contents); }
    }
    Ok(modules)
}

fn parse_link_attribute(line: &str, default_language: &str, _package: &Path) -> Result<Option<NativeLink>> {
    let Some(args) = line.strip_prefix("#[link(").and_then(|v| v.strip_suffix(")]")) else {
        bail!("Invalid link attribute: {line}");
    };
    let args = args.split(',').map(|part| part.trim().trim_matches('"')).collect::<Vec<_>>();
    if args.len() != 3 { bail!("link attribute expects language, source path, and target function: {line}"); }
    let language = args[0];
    if !matches!(language, "c" | "cpp") { bail!("Unsupported link language `{language}`"); }
    let source = Path::new(args[1]);
    if source.is_absolute() || source.components().any(|component| matches!(component, std::path::Component::ParentDir)) {
        bail!("Native source path must remain inside the imported package: {}", source.display());
    }
    if args[2].is_empty() { bail!("Link target function cannot be empty"); }
    Ok(Some(NativeLink {
        language: language.to_owned(),
        package_alias: default_language.to_owned(),
        source_path: source.to_string_lossy().into_owned(),
        target: args[2].to_owned(),
        return_type: String::new(),
    }))
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
