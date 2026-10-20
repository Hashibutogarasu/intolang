use ast::Program;
use errors::{Errors, Result};
use std::path::Path;

use super::builtins::{SOURCE_ROOT, TEST_MACRO};

/// A function marked with the test attribute.
pub struct TestCase {
    pub name: String,
    line: usize,
}

fn parse_module(package: &Path, module: &str) -> Result<Program> {
    let path = package.join(SOURCE_ROOT).join(format!("{module}.into"));
    let source = std::fs::read_to_string(&path).map_err(|error| Errors::read(&path, error))?;
    source
        .parse::<Program>()
        .map_err(|error| error.bound_to(&path.display().to_string()))
}

/// The attribute name the builtin `test` macro declares.
pub fn test_attribute(package: &Path) -> Result<String> {
    let program = parse_module(package, TEST_MACRO.module)?;
    program
        .macros()
        .find(|decl| decl.name == TEST_MACRO.function)
        .and_then(|decl| decl.attributes.first())
        .map(|attribute| attribute.name.clone())
        .ok_or_else(|| Errors::TestMacroNoAnnotation.into())
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
            line: function.line,
        })
        .collect::<Vec<_>>();
    tests.sort_by_key(|test| test.line);
    tests
}
