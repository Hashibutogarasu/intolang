use anyhow::{Result, bail};
use ast::Program;
use std::collections::HashSet;

use super::public_items::public_items;
use crate::module::LoadedModule;
use crate::spec::ExportSpec;

/// Collect the names `alias://module` exposes, honouring `pub` visibility.
///
/// A path that names an existing module file exports every `pub` declaration in
/// that module. Otherwise the last path segment is an explicit item export,
/// which lets packages expose symbols without a backing source file.
pub fn module_exports(info: &Program, loaded: &LoadedModule) -> Result<HashSet<String>> {
    let module = &loaded.module;
    let mut names = HashSet::new();
    for decl in info.exports() {
        let Ok(spec) = ExportSpec::try_from(decl) else {
            continue;
        };
        if spec.alias != module.alias {
            continue;
        }
        if spec
            .path
            .split('/')
            .any(|part| part.is_empty() || part == "..")
        {
            bail!("Invalid export path in into-info.into: {}", spec.path);
        }
        let module_source = module
            .package
            .root
            .join("src/main")
            .join(format!("{}.into", spec.path));
        if module_source.is_file() {
            if spec.path == module.name {
                names.extend(public_items(&loaded.program));
            }
        } else if let Some((export_module, item)) = spec.path.rsplit_once('/')
            && export_module == module.name
        {
            names.insert(item.to_owned());
        }
    }
    Ok(names)
}
