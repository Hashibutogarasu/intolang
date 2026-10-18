use anyhow::Result;
use ast::Program;
use native::NativeLink;
use std::collections::HashMap;
use std::path::Path;

use crate::exports::public_items;
use crate::module::collect_modules;

/// Read `#[link("c|cpp", path, symbol)]` metadata attached to the public
/// foreign declarations of every imported module. `path` is relative to the
/// imported package root.
pub fn resolve_links(
    program: &Program,
    packages_dir: &Path,
) -> Result<HashMap<String, NativeLink>> {
    let modules = collect_modules(program, packages_dir)?;
    let mut links = HashMap::new();
    for loaded in &modules {
        let public = public_items(&loaded.program);
        let alias = &loaded.module.alias;
        for function in loaded.program.functions() {
            if let Some(link) = NativeLink::from_function(function, alias)?
                && public.contains(&function.name)
            {
                links.insert(function.name.clone(), link);
            }
        }
        for class in loaded.program.classes() {
            for method in &class.methods {
                let key = format!("{}.{}", class.name, method.name);
                if let Some(link) = NativeLink::from_function(method, alias)?
                    && public.contains(&key)
                {
                    links.insert(key, link);
                }
            }
        }
    }
    Ok(links)
}
