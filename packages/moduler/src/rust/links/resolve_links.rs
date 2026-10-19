use anyhow::Result;
use ast::{FunctionDecl, Program, overload_key};
use native::NativeLink;
use std::collections::HashMap;
use std::path::Path;

use crate::exports::public_items;
use crate::module::collect_modules;

/// The key a declaration is stored under: its name, or the name with the
/// types of its parameters when another declaration of `declared` shares it.
fn link_key(base: String, function: &FunctionDecl, declared: &[&FunctionDecl]) -> String {
    let shared = declared
        .iter()
        .filter(|other| other.name == function.name)
        .count();
    if shared > 1 {
        overload_key(&base, &function.params)
    } else {
        base
    }
}

/// Read `#[link("c|cpp", path, symbol)]` metadata attached to the public
/// foreign declarations of every imported module. `path` is relative to the
/// imported package root. Functions that share a name and differ in the types
/// of their parameters are stored under `name$Type$Type`.
pub fn resolve_links(
    program: &Program,
    packages_dir: &Path,
) -> Result<HashMap<String, NativeLink>> {
    let modules = collect_modules(program, packages_dir)?;
    let mut links = HashMap::new();
    for loaded in &modules {
        let public = public_items(&loaded.program);
        let alias = &loaded.module.alias;
        let functions: Vec<&FunctionDecl> = loaded.program.functions().collect();
        for function in &functions {
            if let Some(link) = NativeLink::from_function(function, alias)?
                && public.contains(&function.name)
            {
                let key = link_key(function.name.clone(), function, &functions);
                links.insert(key, link);
            }
        }
        for class in loaded.program.classes() {
            let methods: Vec<&FunctionDecl> = class.methods.iter().collect();
            for method in &methods {
                let base = format!("{}.{}", class.name, method.name);
                if let Some(link) = NativeLink::from_function(method, alias)?
                    && public.contains(&base)
                {
                    let key = link_key(base, method, &methods);
                    links.insert(key, link);
                }
            }
        }
    }
    Ok(links)
}
