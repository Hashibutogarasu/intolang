//! Syntax analysis: parsing and formatting of Into source.
//!
//! The directory is organised so that each syntactic construct lives in its
//! own module under [`constructs`], shared lowering primitives live under
//! [`support`], and the [`Parser`] itself only runs an ordered construct list.

mod context;
mod format;
mod formatter;
mod parse;
mod parser;

pub mod constructs;
pub mod support;

pub use formatter::format_source;
pub use parser::Parser;

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::syntax::context::ParseContext;
use crate::syntax::parse::Line;
use crate::syntax::support::{
    collect_type_params, parse_classes, parse_enums, parse_user_functions,
};
use crate::{imports, lexer::tokenize, validation};

/// Parse and lower an Into source file to its observable output.
pub fn lower(source: &str, packages_dir: Option<&Path>) -> Result<String> {
    lower_named(source, packages_dir, "<source>")
}

/// Resolve imports against bundled packages, then parse and lower the source.
pub fn lower_named(source: &str, packages_dir: Option<&Path>, source_path: &str) -> Result<String> {
    let _tokens = tokenize(source)?;
    validation::function_scopes(source, source_path)?;

    let imported_functions = if let Some(packages_dir) = packages_dir {
        imports::resolve(source, packages_dir)?
    } else {
        Default::default()
    };
    let native_links = if let Some(packages_dir) = packages_dir {
        imports::resolve_links(source, packages_dir)?
    } else {
        HashMap::new()
    };
    let type_params = collect_type_params(source);
    let (entry_enums, _) = parse_enums(source, source_path)?;
    let module_sources = if let Some(packages_dir) = packages_dir {
        imports::imported_module_sources(source, packages_dir)?
    } else {
        Vec::new()
    };

    let mut ctx = ParseContext::new(source_path);
    ctx.imported_functions = imported_functions;
    ctx.native_links = native_links;
    ctx.type_params = type_params;

    for module in &module_sources {
        let (module_enums, _) = parse_enums(module, source_path)?;
        for (name, variants) in module_enums {
            ctx.enum_values.entry(name).or_insert(variants);
        }
        for (name, fields) in parse_classes(module) {
            ctx.class_fields.insert(name, fields);
        }
        for function in parse_user_functions(module) {
            ctx.user_functions.insert(function.name.clone(), function);
        }
    }
    for (name, variants) in entry_enums {
        ctx.enum_values.insert(name, variants);
    }
    for function in parse_user_functions(source) {
        ctx.user_functions.insert(function.name.clone(), function);
    }

    for (class, fields) in &ctx.class_fields {
        ctx.output.push('\u{1e}');
        ctx.output.push_str("NATIVE_CLASS\t");
        ctx.output.push_str(class);
        for (field, ty) in fields {
            ctx.output.push('\t');
            ctx.output.push_str(field);
            ctx.output.push(':');
            ctx.output.push_str(ty);
        }
        ctx.output.push('\n');
    }

    let lines = prepare_lines(source);
    let parser = Parser::new(constructs::default_constructs());
    parser.parse(&mut ctx, &lines)?;
    Ok(ctx.output)
}

fn prepare_lines(source: &str) -> Vec<Line> {
    source
        .lines()
        .enumerate()
        .map(|(index, raw)| Line::new(index + 1, raw))
        .collect()
}
