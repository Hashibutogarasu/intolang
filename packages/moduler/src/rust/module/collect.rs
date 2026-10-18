use anyhow::{Context, Result};
use ast::Program;
use std::collections::HashSet;
use std::path::Path;

use super::loaded::LoadedModule;
use super::module::Module;
use crate::package::{PackageInfo, find_by_alias};
use crate::spec::ImportSpec;

/// Follow `import` statements depth-first, returning every reachable module
/// after the modules it imports.
fn collect(
    program: &Program,
    packages_dir: &Path,
    seen: &mut HashSet<String>,
    out: &mut Vec<LoadedModule>,
) -> Result<()> {
    for decl in program.imports() {
        let Ok(spec) = ImportSpec::try_from(decl) else {
            continue;
        };
        if !seen.insert(spec.to_string()) {
            continue;
        }
        let Some(root) = find_by_alias(packages_dir, &spec.alias)? else {
            continue;
        };
        let module_path = root.join("src/main").join(format!("{}.into", spec.module));
        let contents = std::fs::read_to_string(&module_path).unwrap_or_default();
        let module_program = contents
            .parse::<Program>()
            .with_context(|| format!("Could not parse module {}", module_path.display()))?;
        collect(&module_program, packages_dir, seen, out)?;
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
            program: module_program,
        });
    }
    Ok(())
}

/// Follow the imports reachable from `program`, returning every module.
pub fn collect_modules(program: &Program, packages_dir: &Path) -> Result<Vec<LoadedModule>> {
    let mut modules = Vec::new();
    collect(program, packages_dir, &mut HashSet::new(), &mut modules)?;
    Ok(modules)
}
