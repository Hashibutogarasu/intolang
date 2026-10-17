use anyhow::{Result, anyhow, bail};
use std::{collections::HashSet, path::Path};

use crate::syntax::support::text::{brace_delta, is_identifier, strip_line_comment};

/// A module reached from the entry source, directly or transitively.
struct ImportedModule {
    alias: String,
    module: String,
    package: std::path::PathBuf,
    contents: String,
}

fn parse_import(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    let uri = line.strip_prefix("import ")?.strip_prefix('"')?;
    let uri = uri.split_once('"')?.0;
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
    Some((alias, module))
}

/// Follow `import` statements breadth-first, returning every reachable module.
fn collect_modules(
    source: &str,
    packages_dir: &Path,
    seen: &mut HashSet<String>,
    out: &mut Vec<ImportedModule>,
) -> Result<()> {
    for line in source.lines() {
        let Some((alias, module)) = parse_import(line) else {
            continue;
        };
        let key = format!("{alias}://{module}");
        if !seen.insert(key) {
            continue;
        }
        let Some(package) = find_by_alias(packages_dir, &alias)? else {
            continue;
        };
        let module_path = package.join("src/main").join(format!("{module}.into"));
        let contents = std::fs::read_to_string(&module_path).unwrap_or_default();
        collect_modules(&contents, packages_dir, seen, out)?;
        out.push(ImportedModule {
            alias,
            module,
            package,
            contents,
        });
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub(super) struct Param {
    pub name: String,
    pub default: Option<String>,
}

/// Parse a comma separated parameter list, honouring optional markers written
/// as `name: Type?` or `name: Type = <literal>`.
pub(super) fn parse_params(text: &str) -> Vec<Param> {
    let mut params = Vec::new();
    for part in text
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let (declaration, value) = match part.split_once('=') {
            Some((declaration, value)) => (declaration.trim(), Some(value.trim())),
            None => (part, None),
        };
        let (name, type_text) = match declaration.split_once(':') {
            Some((name, type_text)) => (name.trim(), type_text.trim()),
            None => (declaration, ""),
        };
        let (type_text, marker) = match type_text.strip_suffix('?') {
            Some(inner) => (inner.trim(), true),
            None => (type_text, false),
        };
        let default = match value {
            Some(value) => Some(normalize_default(value)),
            None if marker => Some(default_for_type(type_text)),
            None => None,
        };
        params.push(Param {
            name: name.to_owned(),
            default,
        });
    }
    params
}

fn normalize_default(value: &str) -> String {
    match value {
        "true" => "1".to_owned(),
        "false" => "0".to_owned(),
        other => other.to_owned(),
    }
}

fn default_for_type(type_text: &str) -> String {
    match type_text {
        "String" | "string" | "str" => "\"\"".to_owned(),
        _ => "0".to_owned(),
    }
}

#[derive(Debug, Clone)]
pub(super) struct NativeLink {
    pub language: String,
    pub package_alias: String,
    pub source_path: String,
    pub target: String,
    pub return_type: String,
    pub params: Vec<Param>,
}

/// Resolve Into package and native-language module imports using package exports.
///
/// Imports are followed transitively so a module may itself import other
/// packages (for example `ffmpeg` importing `fs`).
pub(super) fn resolve(source: &str, packages_dir: &Path) -> Result<HashSet<String>> {
    let mut modules = Vec::new();
    collect_modules(source, packages_dir, &mut HashSet::new(), &mut modules)?;
    let mut imported_functions = HashSet::new();
    for module in &modules {
        let info_path = module.package.join("src/main/into-info.into");
        let info = std::fs::read_to_string(&info_path)
            .map_err(|_| anyhow!("Package `{}` has no into-info.into", module.alias))?;
        let expected = format!("{}/{}/", module.alias, module.module);
        let exact_export = format!("{}/{}.", module.alias, module.module);
        let exports = info
            .lines()
            .filter_map(|decl| {
                let export = decl.trim().strip_prefix("pub ")?;
                let function = if export.trim_end_matches(';') == exact_export {
                    module
                        .module
                        .rsplit('/')
                        .next()
                        .unwrap_or(module.module.as_str())
                } else {
                    export
                        .strip_prefix(&expected)?
                        .trim_end_matches(';')
                        .trim_end_matches('.')
                };
                (!function.is_empty()).then(|| function.to_owned())
            })
            .collect::<Vec<_>>();
        if exports.is_empty() {
            bail!(
                "Package `{}` does not export module `{}`",
                module.alias,
                module.module
            );
        }
        imported_functions.extend(exports);
    }
    Ok(imported_functions)
}

/// Read `#[link("c|cpp", path, symbol)]` metadata attached to imported
/// foreign declarations. `path` is relative to the imported package root.
pub(super) fn resolve_links(
    source: &str,
    packages_dir: &Path,
) -> Result<std::collections::HashMap<String, NativeLink>> {
    let mut modules = Vec::new();
    collect_modules(source, packages_dir, &mut HashSet::new(), &mut modules)?;
    let mut links = std::collections::HashMap::new();
    for module in &modules {
        let lines = module.contents.lines().collect::<Vec<_>>();
        let mut stack: Vec<(String, i32)> = Vec::new();
        let mut depth = 0i32;
        let mut pending: Option<(NativeLink, Option<String>)> = None;
        for raw in &lines {
            while let Some((_, level)) = stack.last() {
                if depth <= *level {
                    stack.pop();
                } else {
                    break;
                }
            }
            let line = strip_line_comment(raw).trim();
            let current_class = stack.last().map(|(name, _)| name.clone());
            if line.starts_with("#[link(") {
                pending = parse_link_attribute(line, &module.alias, &module.package)?
                    .map(|link| (link, current_class.clone()));
                continue;
            }
            if let Some((link, owner)) = pending.take() {
                if let Some(rest) = line.split_once("fun ").map(|(_, rest)| rest) {
                    if let Some((name, _)) = rest.split_once('(') {
                        let return_type = rest
                            .rsplit_once("):")
                            .map(|(_, ty)| ty.trim().trim_end_matches(';').trim().to_owned())
                            .unwrap_or_else(|| "Void".into());
                        let params_text = rest
                            .split_once('(')
                            .map(|(_, after)| after)
                            .and_then(|after| after.rsplit_once("):").map(|(params, _)| params))
                            .unwrap_or("");
                        let mut link = link;
                        link.return_type = return_type;
                        link.params = parse_params(params_text);
                        let name = name.trim();
                        let key = match owner.or(current_class) {
                            Some(class) => format!("{class}.{name}"),
                            None => name.to_owned(),
                        };
                        links.insert(key, link);
                    }
                }
            }
            if let Some((prefix, _)) = line.split_once("() {") {
                let name = prefix.strip_prefix("pub ").unwrap_or(prefix).trim();
                if is_identifier(name) {
                    stack.push((name.to_owned(), depth));
                }
            }
            depth += brace_delta(line);
        }
    }
    Ok(links)
}

pub(super) fn imported_module_sources(source: &str, packages_dir: &Path) -> Result<Vec<String>> {
    let mut modules = Vec::new();
    collect_modules(source, packages_dir, &mut HashSet::new(), &mut modules)?;
    Ok(modules
        .into_iter()
        .map(|module| module.contents)
        .filter(|contents| !contents.is_empty())
        .collect())
}

fn parse_link_attribute(
    line: &str,
    default_language: &str,
    _package: &Path,
) -> Result<Option<NativeLink>> {
    let Some(args) = line
        .strip_prefix("#[link(")
        .and_then(|v| v.strip_suffix(")]"))
    else {
        bail!("Invalid link attribute: {line}");
    };
    let args = args
        .split(',')
        .map(|part| part.trim().trim_matches('"'))
        .collect::<Vec<_>>();
    if args.len() != 3 {
        bail!("link attribute expects language, source path, and target function: {line}");
    }
    let language = args[0];
    if !matches!(language, "c" | "cpp") {
        bail!("Unsupported link language `{language}`");
    }
    let source = Path::new(args[1]);
    if source.is_absolute()
        || source
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        bail!(
            "Native source path must remain inside the imported package: {}",
            source.display()
        );
    }
    if args[2].is_empty() {
        bail!("Link target function cannot be empty");
    }
    Ok(Some(NativeLink {
        language: language.to_owned(),
        package_alias: default_language.to_owned(),
        source_path: source.to_string_lossy().into_owned(),
        target: args[2].to_owned(),
        return_type: String::new(),
        params: Vec::new(),
    }))
}

pub(super) fn find_by_alias(
    packages_dir: &Path,
    alias: &str,
) -> Result<Option<std::path::PathBuf>> {
    for entry in std::fs::read_dir(packages_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let manifest_path = entry.path().join("into-package.json");
        let Ok(contents) = std::fs::read_to_string(manifest_path) else {
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
