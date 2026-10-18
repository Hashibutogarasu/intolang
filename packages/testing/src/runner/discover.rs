use anyhow::{Context, Result, anyhow};
use ast::{FunctionDecl, Program, Stmt};
use std::collections::HashMap;
use std::path::Path;

use super::builtins::{SOURCE_ROOT, TEST_MACRO};

/// A function marked with the test attribute.
pub struct TestCase {
    pub name: String,
    pub body: Vec<Stmt>,
    line: usize,
}

fn parse_module(package: &Path, module: &str) -> Result<Program> {
    let path = package.join(SOURCE_ROOT).join(format!("{module}.into"));
    let source = std::fs::read_to_string(&path)
        .with_context(|| format!("Could not read {}", path.display()))?;
    source
        .parse::<Program>()
        .with_context(|| format!("Could not parse {}", path.display()))
}

/// The attribute name the builtin `test` macro declares.
pub fn test_attribute(package: &Path) -> Result<String> {
    let program = parse_module(package, TEST_MACRO.module)?;
    program
        .macros()
        .find(|decl| decl.name == TEST_MACRO.function)
        .and_then(|decl| decl.attributes.first())
        .map(|attribute| attribute.name.clone())
        .ok_or_else(|| anyhow!("test macro does not define the #[test] annotation"))
}

/// The public functions with bodies that the builtin `module` defines.
pub fn load_definitions(package: &Path, module: &str) -> Result<HashMap<String, FunctionDecl>> {
    Ok(parse_module(package, module)?
        .functions()
        .filter(|function| function.is_public && function.body.is_some())
        .map(|function| (function.name.clone(), function.clone()))
        .collect())
}

/// The functions of `program` carrying `attribute`, in source order.
pub fn find_tests(program: &Program, attribute: &str) -> Vec<TestCase> {
    let methods = program.classes().flat_map(|class| class.methods.iter());
    let mut tests = program
        .functions()
        .chain(methods)
        .filter(|function| {
            function
                .attributes
                .iter()
                .any(|candidate| candidate.name == attribute)
        })
        .map(|function| TestCase {
            name: function.name.clone(),
            body: function.body.clone().unwrap_or_default(),
            line: function.line,
        })
        .collect::<Vec<_>>();
    tests.sort_by_key(|test| test.line);
    tests
}
