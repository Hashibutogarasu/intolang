use anyhow::{Result, anyhow, bail};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::items::public_items;
use super::module::{Module, parse_export, parse_import};
use super::package::{PackageInfo, find_by_alias};
use super::text::{brace_delta, is_identifier, strip_line_comment};

/// A module together with the source text that was read for it.
pub struct LoadedModule {
    pub module: Module,
    pub contents: String,
}

impl LoadedModule {
    /// The package the module belongs to.
    pub fn package(&self) -> &PackageInfo {
        &self.module.package
    }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub default: Option<String>,
}

/// Parse a comma separated parameter list, honouring optional markers written
/// as `name: Type?` or `name: Type = <literal>`.
pub fn parse_params(text: &str) -> Vec<Param> {
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
pub struct NativeLink {
    pub language: String,
    pub package_alias: String,
    pub source_path: String,
    pub target: String,
    pub return_type: String,
    pub params: Vec<Param>,
}

/// Follow `import` statements breadth-first, returning every reachable module.
fn collect(
    source: &str,
    packages_dir: &Path,
    seen: &mut HashSet<String>,
    out: &mut Vec<LoadedModule>,
) -> Result<()> {
    for line in source.lines() {
        let Some(spec) = parse_import(line) else {
            continue;
        };
        let key = spec.uri();
        if !seen.insert(key) {
            continue;
        }
        let Some(root) = find_by_alias(packages_dir, &spec.alias)? else {
            continue;
        };
        let module_path = root.join("src/main").join(format!("{}.into", spec.module));
        let contents = std::fs::read_to_string(&module_path).unwrap_or_default();
        collect(&contents, packages_dir, seen, out)?;
        let manifest_contents = std::fs::read_to_string(root.join("into-package.json"))?;
        let manifest: serde_json::Value = serde_json::from_str(&manifest_contents)?;
        let package = PackageInfo::new(&spec.alias, root, &manifest);
        out.push(LoadedModule {
            module: Module {
                package,
                source_path: module_path,
                alias: spec.alias,
                name: spec.module,
            },
            contents,
        });
    }
    Ok(())
}

/// Follow the imports reachable from `source`, returning every module.
pub fn collect_modules(source: &str, packages_dir: &Path) -> Result<Vec<LoadedModule>> {
    let mut modules = Vec::new();
    collect(source, packages_dir, &mut HashSet::new(), &mut modules)?;
    Ok(modules)
}

/// Resolve Into package and native-language module imports using package exports.
///
/// Imports are followed transitively so a module may itself import other
/// packages (for example `ffmpeg` importing `fs`).
pub fn resolve(source: &str, packages_dir: &Path) -> Result<HashSet<String>> {
    let modules = collect_modules(source, packages_dir)?;
    let mut imported_functions = HashSet::new();
    for loaded in &modules {
        let info_path = loaded.package().root.join("src/main/into-info.into");
        let info = std::fs::read_to_string(&info_path)
            .map_err(|_| anyhow!("Package `{}` has no into-info.into", loaded.module.alias))?;
        let exports = module_exports(&info, loaded)?;
        if exports.is_empty() {
            bail!(
                "Package `{}` does not export module `{}`",
                loaded.module.alias,
                loaded.module.name
            );
        }
        imported_functions.extend(exports);
    }
    Ok(imported_functions)
}

/// Collect the names `alias://module` exposes, honouring `pub` visibility.
///
/// A path that names an existing module file exports every `pub` declaration in
/// that module. Otherwise the last path segment is an explicit item export,
/// which lets packages expose symbols without a backing source file.
pub fn module_exports(info: &str, loaded: &LoadedModule) -> Result<HashSet<String>> {
    let module = &loaded.module;
    let mut names = HashSet::new();
    for line in info.lines() {
        let Some(spec) = parse_export(line) else {
            continue;
        };
        if spec.alias != module.alias {
            continue;
        }
        if spec.path.split('/').any(|part| part.is_empty() || part == "..") {
            bail!("Invalid export path in into-info.into: {}", spec.path);
        }
        let module_source = module
            .package
            .root
            .join("src/main")
            .join(format!("{}.into", spec.path));
        if module_source.is_file() {
            if spec.path == module.name {
                names.extend(public_items(&loaded.contents));
            }
        } else if let Some((export_module, item)) = spec.path.rsplit_once('/') {
            if export_module == module.name {
                names.insert(item.to_owned());
            }
        }
    }
    Ok(names)
}

/// Read `#[link("c|cpp", path, symbol)]` metadata attached to imported
/// foreign declarations. `path` is relative to the imported package root.
pub fn resolve_links(source: &str, packages_dir: &Path) -> Result<HashMap<String, NativeLink>> {
    let modules = collect_modules(source, packages_dir)?;
    let mut links = HashMap::new();
    for loaded in &modules {
        let public = public_items(&loaded.contents);
        let lines = loaded.contents.lines().collect::<Vec<_>>();
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
                pending =
                    parse_link_attribute(line, &loaded.module.alias, &loaded.module.package.root)?
                        .map(|link| (link, current_class.clone()));
                continue;
            }
            if let Some((link, owner)) = pending.take() {
                if let Some((_, rest)) = line.split_once("fun ") {
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
                        if public.contains(&key) {
                            links.insert(key, link);
                        }
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

pub fn imported_module_sources(source: &str, packages_dir: &Path) -> Result<Vec<String>> {
    let modules = collect_modules(source, packages_dir)?;
    Ok(modules
        .into_iter()
        .map(|loaded| loaded.contents)
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
