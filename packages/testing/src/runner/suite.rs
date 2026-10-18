use anyhow::{Result, anyhow, bail};
use ast::{Program, SyntaxError};
use parser::PathLexeme;
use std::collections::HashMap;
use std::path::Path;

use super::builtins::{ASSERTIONS, BUILTIN_ALIAS, TEST_MACRO};
use super::discover::{find_tests, load_definitions, test_attribute};
use super::interpreter::Interpreter;
use super::summary::TestSummary;

/// Run all `#[test]` functions in one Into source file.
pub fn run_tests(source: &str, path: &Path, packages_dir: &Path) -> Result<TestSummary> {
    let path_text = PathLexeme::from(path).to_string();
    let program = source.parse::<Program>().map_err(|error| {
        match error.downcast_ref::<SyntaxError>() {
            Some(syntax) => anyhow!("{}", syntax.backtrace(&path_text)),
            None => error,
        }
    })?;
    let imported = moduler::resolve(&program, packages_dir)?;
    let package = moduler::find_by_alias(packages_dir, BUILTIN_ALIAS)?
        .ok_or_else(|| anyhow!("Builtin package is not bundled"))?;
    let mut functions = HashMap::new();
    for required in &ASSERTIONS {
        if !imported.contains(required.function) {
            bail!(
                "{path_text}: test module must import `{}`",
                required.function
            );
        }
        functions.extend(load_definitions(&package, required.module)?);
    }
    if !imported.contains(TEST_MACRO.function) {
        bail!("{path_text}: test module must import the test macro");
    }
    let attribute = test_attribute(&package)?;

    let mut summary = TestSummary::default();
    for test in find_tests(&program, &attribute) {
        let interpreter = Interpreter {
            functions: &functions,
            path: &path_text,
            test: &test.name,
        };
        match interpreter.run_test(&test.body) {
            Ok(()) => {
                summary.passed += 1;
                println!("test {} ... ok", test.name);
            }
            Err(error) => {
                summary.failed += 1;
                eprintln!("{error:#}");
            }
        }
    }
    if summary.passed + summary.failed == 0 {
        bail!("{path_text}: no #[test] functions found");
    }
    Ok(summary)
}
