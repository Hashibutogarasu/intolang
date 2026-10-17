use anyhow::{Context, Result, bail};
use std::path::Path;

use super::text::strip_keyword;

/// A function exposed from a native library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeFunction {
    pub name: String,
    pub output: String,
}

/// Resolve the modules exported by `lib.into` and collect their public functions.
pub fn lower_library(source: &str, entry: &Path, _packages_dir: &Path) -> Result<Vec<NativeFunction>> {
    let module_root = entry.parent().context("Library entry has no parent")?;
    let mut functions = Vec::new();
    for module in exported_modules(source)? {
        let module_path = module_root.join(format!("{module}.into"));
        let module_source = std::fs::read_to_string(&module_path)
            .with_context(|| format!("Could not read exported module {}", module_path.display()))?;
        functions.extend(public_functions(&module_source));
    }
    if functions.is_empty() {
        bail!("Library entry exports no public functions");
    }
    Ok(functions)
}

fn exported_modules(source: &str) -> Result<Vec<String>> {
    let mut modules = Vec::new();
    for line in source.lines().map(str::trim) {
        let Some(rest) = strip_keyword(line, "export") else {
            continue;
        };
        let Some(uri) = rest.trim_start().strip_prefix('"') else {
            continue;
        };
        let uri = uri.split_once('"').map(|(uri, _)| uri).unwrap_or(uri);
        let Some((_, module)) = uri.split_once("://") else {
            bail!("Invalid library export URI: {uri}");
        };
        if module.is_empty() || module.split('/').any(|part| part == ".." || part.is_empty()) {
            bail!("Invalid library export module: {uri}");
        }
        modules.push(module.to_owned());
    }
    if modules.is_empty() {
        bail!("Library entry does not export any modules");
    }
    Ok(modules)
}

fn public_functions(source: &str) -> Vec<NativeFunction> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut functions = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim();
        let Some(without_visibility) = strip_keyword(line, "pub") else {
            index += 1;
            continue;
        };
        let header = strip_keyword(without_visibility, "static").unwrap_or(without_visibility);
        if let Some(rest) = strip_keyword(header, "fun") {
            let Some(open_paren) = rest.find('(') else {
                index += 1;
                continue;
            };
            let name = rest[..open_paren].split('<').next().unwrap_or("").trim();
            if name.is_empty() {
                index += 1;
                continue;
            }
            let mut body = String::new();
            let mut depth = 0usize;
            let mut entered_body = false;
            while index < lines.len() {
                let current = lines[index];
                if current.contains('{') {
                    depth += current.matches('{').count();
                    entered_body = true;
                }
                if entered_body {
                    if let Some((_, rest)) = current.split_once("print(") {
                        if let Some(value) = rest
                            .trim_start()
                            .strip_prefix('"')
                            .and_then(|value| value.split_once('"').map(|(value, _)| value))
                        {
                            body.push_str(value);
                            body.push('\n');
                        }
                    }
                    depth = depth.saturating_sub(current.matches('}').count());
                    if depth == 0 {
                        index += 1;
                        break;
                    }
                }
                index += 1;
            }
            functions.push(NativeFunction {
                name: name.to_owned(),
                output: body,
            });
            continue;
        }
        index += 1;
    }
    functions
}
