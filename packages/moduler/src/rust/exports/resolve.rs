use anyhow::{Context, Result, anyhow, bail};
use ast::Program;
use std::collections::HashSet;
use std::path::Path;

use super::module_exports::module_exports;
use crate::module::collect_modules;

/// Resolve Into package and native-language module imports using package exports.
///
/// Imports are followed transitively so a module may itself import other
/// packages (for example `ffmpeg` importing `fs`).
pub fn resolve(program: &Program, packages_dir: &Path) -> Result<HashSet<String>> {
    let modules = collect_modules(program, packages_dir)?;
    let mut imported_functions = HashSet::new();
    for loaded in &modules {
        let info_path = loaded.package().root.join("src/main/into-info.into");
        let info = std::fs::read_to_string(&info_path)
            .map_err(|_| anyhow!("Package `{}` has no into-info.into", loaded.module.alias))?;
        let info = info
            .parse::<Program>()
            .with_context(|| format!("Could not parse {}", info_path.display()))?;
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
