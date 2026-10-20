use ast::{Program, UseDecl};
use errors::{Error, Errors, Result};
use std::path::{Path, PathBuf};

use crate::package::{PackageInfo, find_by_alias};
use crate::spec::{ExportSpec, ImportSpec};

const INFO_FILE: &str = "src/main/into-info.into";

/// The source file an `import` names, when its package and module exist.
pub fn import_path(decl: &UseDecl, packages_dir: &Path) -> Result<Option<PathBuf>> {
    let spec = ImportSpec::try_from(decl)?;
    let Some(root) = find_by_alias(packages_dir, &spec.alias)? else {
        return Ok(None);
    };
    let path = root.join("src/main").join(format!("{}.into", spec.module));
    Ok(path.is_file().then_some(path))
}

/// The package an `import` or `export` names and the file that declares the
/// module, or the file that lists its exports when the module has none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportTarget {
    pub package: PackageInfo,
    pub file: Option<PathBuf>,
}

/// The package and file of an `import`, when its package exists.
pub fn import_target(decl: &UseDecl, packages_dir: &Path) -> Result<Option<ImportTarget>> {
    let spec = ImportSpec::try_from(decl)?;
    target(packages_dir, &spec.alias, &spec.module)
}

/// The package and file of an `export`, when its package exists. The path may
/// name an item of a module, so it is shortened until a file answers.
pub fn export_target(decl: &UseDecl, packages_dir: &Path) -> Result<Option<ImportTarget>> {
    let spec = ExportSpec::try_from(decl)?;
    target(packages_dir, &spec.alias, &spec.path)
}

fn target(packages_dir: &Path, alias: &str, module: &str) -> Result<Option<ImportTarget>> {
    let Some(root) = find_by_alias(packages_dir, alias)? else {
        return Ok(None);
    };
    let manifest = std::fs::read_to_string(root.join("into-package.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .unwrap_or_default();
    let package = PackageInfo::new(alias, root.clone(), &manifest);
    let mut path = module;
    let file = loop {
        let candidate = root.join("src/main").join(format!("{path}.into"));
        if candidate.is_file() {
            break Some(candidate);
        }
        match path.rsplit_once('/') {
            Some((parent, _)) => path = parent,
            None => break None,
        }
    };
    let file = file.or_else(|| {
        let info = root.join(INFO_FILE);
        info.is_file().then_some(info)
    });
    Ok(Some(ImportTarget { package, file }))
}

/// Whether the package exports names under `module` without a source file for
/// it, as the builtin package does for `std`.
fn exported_without_file(root: &Path, alias: &str, module: &str) -> bool {
    let Ok(info) = std::fs::read_to_string(root.join(INFO_FILE)) else {
        return false;
    };
    let Ok(program) = info.parse::<Program>() else {
        return false;
    };
    program
        .exports()
        .filter_map(|decl| ExportSpec::try_from(decl).ok())
        .filter(|spec| spec.alias == alias)
        .any(|spec| {
            spec.path == module
                || spec
                    .path
                    .strip_prefix(module)
                    .is_some_and(|rest| rest.starts_with('/'))
        })
}

fn resolves(decl: &UseDecl, packages_dir: &Path) -> Result<bool> {
    let spec = ImportSpec::try_from(decl)?;
    let Some(root) = find_by_alias(packages_dir, &spec.alias)? else {
        return Ok(false);
    };
    let file = root.join("src/main").join(format!("{}.into", spec.module));
    Ok(file.is_file() || exported_without_file(&root, &spec.alias, &spec.module))
}

/// The imports of `program` that name no package or no module, with the
/// error that says why.
pub fn unresolved_imports<'a>(
    program: &'a Program,
    packages_dir: &Path,
) -> Vec<(&'a UseDecl, Error)> {
    let mut unresolved = Vec::new();
    for decl in program.imports() {
        match resolves(decl, packages_dir) {
            Ok(true) => {}
            Ok(false) => unresolved.push((
                decl,
                Errors::ImportNotFound {
                    uri: decl.uri.clone(),
                }
                .into(),
            )),
            Err(error) => unresolved.push((decl, error)),
        }
    }
    unresolved
}
