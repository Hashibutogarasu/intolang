use anyhow::{Context, Result, bail};
use ast::{Expr, FunctionDecl, Program, Stmt, StmtKind};
use std::path::Path;
use types::Literal;

use super::native_function::NativeFunction;
use crate::spec::ExportSpec;

/// Resolve the modules exported by `lib.into` and collect their public functions.
pub fn lower_library(
    program: &Program,
    entry: &Path,
    _packages_dir: &Path,
) -> Result<Vec<NativeFunction>> {
    let module_root = entry.parent().context("Library entry has no parent")?;
    let mut functions = Vec::new();
    for module in exported_modules(program)? {
        let module_path = module_root.join(format!("{module}.into"));
        let module_source = std::fs::read_to_string(&module_path)
            .with_context(|| format!("Could not read exported module {}", module_path.display()))?;
        let module_program = module_source
            .parse::<Program>()
            .with_context(|| format!("Could not parse {}", module_path.display()))?;
        functions.extend(public_functions(&module_program));
    }
    if functions.is_empty() {
        bail!("Library entry exports no public functions");
    }
    Ok(functions)
}

fn exported_modules(program: &Program) -> Result<Vec<String>> {
    let mut modules = Vec::new();
    for decl in program.exports() {
        let Ok(spec) = ExportSpec::try_from(decl) else {
            bail!("Invalid library export URI: {}", decl.uri);
        };
        if spec
            .path
            .split('/')
            .any(|part| part == ".." || part.is_empty())
        {
            bail!("Invalid library export module: {}", decl.uri);
        }
        modules.push(spec.path);
    }
    if modules.is_empty() {
        bail!("Library entry does not export any modules");
    }
    Ok(modules)
}

fn public_functions(program: &Program) -> Vec<NativeFunction> {
    let methods = program.classes().flat_map(|class| class.methods.iter());
    program
        .functions()
        .chain(methods)
        .filter(|function| function.is_public)
        .filter_map(|function| {
            function.body.as_ref().map(|_| NativeFunction {
                name: function.name.clone(),
                output: printed_text(function),
            })
        })
        .collect()
}

fn printed_text(function: &FunctionDecl) -> String {
    let mut output = String::new();
    if let Some(body) = &function.body {
        collect_prints(body, &mut output);
    }
    output
}

fn collect_prints(body: &[Stmt], output: &mut String) {
    for stmt in body {
        match &stmt.kind {
            StmtKind::Expr(Expr::Call { callee, args }) if callee.as_ident() == Some("print") => {
                if let Some(Expr::Literal(Literal::String(text))) = args.first() {
                    output.push_str(text);
                    output.push('\n');
                }
            }
            StmtKind::If {
                then_body,
                else_body,
                ..
            } => {
                collect_prints(then_body, output);
                if let Some(else_body) = else_body {
                    collect_prints(else_body, output);
                }
            }
            StmtKind::While { body, .. } | StmtKind::Init(body) => collect_prints(body, output),
            _ => {}
        }
    }
}
