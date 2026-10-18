use anyhow::Result;
use ast::{Item, Program};
use parser::PathLexeme;
use std::path::Path;

use crate::collect::{
    collect_classes, collect_enums, collect_type_params, collect_user_functions,
};
use crate::context::{Env, Lowerer};
use crate::diagnostic::syntax_diagnostic;
use crate::emit;

const ANONYMOUS_SOURCE: &str = "<source>";

/// Parse and lower an Into source to its observable output.
pub fn lower(source: &str) -> Result<String> {
    lower_named(source, None, ANONYMOUS_SOURCE)
}

/// Resolve imports against bundled packages, then parse and lower the source.
pub fn lower_with_packages(source: &str, packages_dir: Option<&Path>) -> Result<String> {
    lower_named(source, packages_dir, ANONYMOUS_SOURCE)
}

/// Resolve and lower a source file while retaining its path for diagnostics.
pub fn lower_file(
    source: &str,
    source_path: &Path,
    packages_dir: Option<&Path>,
) -> Result<String> {
    lower_named(source, packages_dir, &PathLexeme::from(source_path).to_string())
}

fn lower_named(source: &str, packages_dir: Option<&Path>, source_path: &str) -> Result<String> {
    let program = source
        .parse::<Program>()
        .map_err(|error| syntax_diagnostic(source_path, error))?;
    let env = prepare(&program, packages_dir, source_path)?;

    let mut lowerer = Lowerer::new(&env);
    emit::module_classes(&mut lowerer.state.output);
    let mut classes = env.class_fields.iter().collect::<Vec<_>>();
    classes.sort_by(|left, right| left.0.cmp(right.0));
    for (name, fields) in classes {
        emit::class(&mut lowerer.state.output, name, fields);
    }
    for item in &program.items {
        match item {
            Item::Import(decl) => lowerer.lower_import(decl),
            Item::Export(decl) => lowerer.lower_export(decl),
            Item::Stmt(stmt) => lowerer.lower_stmt(stmt)?,
            Item::Enum(_) | Item::Class(_) | Item::Function(_) | Item::Macro(_) => {}
        }
    }
    Ok(lowerer.into_output())
}

/// Collect everything lowering reads: the declarations of the entry file and
/// the public declarations of the modules it imports.
fn prepare(program: &Program, packages_dir: Option<&Path>, source_path: &str) -> Result<Env> {
    let mut env = Env::new(source_path);
    let mut loaded = Vec::new();
    if let Some(packages_dir) = packages_dir {
        env.imported_functions = moduler::resolve(program, packages_dir)?;
        env.native_links = moduler::resolve_links(program, packages_dir)?;
        loaded = moduler::collect_modules(program, packages_dir)?;
    }
    env.type_params = collect_type_params(program);
    env.modules = loaded.iter().map(|loaded| loaded.module.clone()).collect();

    for module in loaded.iter().filter(|module| !module.contents.is_empty()) {
        let public = moduler::public_items(&module.program);
        let module_path = PathLexeme::from(module.module.source_path.as_path()).to_string();
        for (name, variants) in collect_enums(&module.program, &module_path)? {
            env.enum_values.entry(name).or_insert(variants);
        }
        for (name, fields) in collect_classes(&module.program) {
            if public.contains(&name) {
                env.class_fields.insert(name, fields);
            }
        }
        for function in collect_user_functions(&module.program) {
            if public.contains(&function.name) {
                env.user_functions.insert(function.name.clone(), function);
            }
        }
    }
    env.enum_values
        .extend(collect_enums(program, source_path)?);
    for (name, fields) in collect_classes(program) {
        env.class_fields.entry(name).or_insert(fields);
    }
    for function in collect_user_functions(program) {
        env.user_functions.insert(function.name.clone(), function);
    }
    Ok(env)
}
