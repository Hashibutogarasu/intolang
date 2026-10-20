use ast::{Program, UseDecl};
use errors::{Error, Errors, Result};
use std::path::{Path, PathBuf};

use crate::package::find_by_alias;
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
