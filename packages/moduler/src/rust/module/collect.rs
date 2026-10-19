use anyhow::{Context, Result};
use ast::Program;
use std::collections::HashSet;
use std::path::Path;

use super::loaded::LoadedModule;
use super::module::Module;
use crate::package::{BUILTIN_ALIAS, PackageInfo, find_by_alias};
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

/// The modules of the builtin package every program imports without naming
/// them: the members of `Object`, `String`, `List`, `Type` and the classes of
/// functions.
const IMPLICIT_MODULES: [&str; 10] = [
    "object", "string", "callback", "list", "type", "memory", "map", "binary", "console", "json",
];
const IMPLICIT_MODULE_FILE: &str = "src/main/object.into";

/// Follow the imports reachable from `program`, returning every module.
pub fn collect_modules(program: &Program, packages_dir: &Path) -> Result<Vec<LoadedModule>> {
    let mut modules = Vec::new();
    let mut seen = HashSet::new();
    let provided = find_by_alias(packages_dir, BUILTIN_ALIAS)?
        .is_some_and(|root| root.join(IMPLICIT_MODULE_FILE).is_file());
    if provided {
        let imports: String = IMPLICIT_MODULES
            .iter()
            .map(|module| format!("import \"intolang://builtin/{module}\";\n"))
            .collect();
        let implicit = imports.parse::<Program>()?;
        collect(&implicit, packages_dir, &mut seen, &mut modules)?;
    }
    collect(program, packages_dir, &mut seen, &mut modules)?;
    Ok(modules)
}
