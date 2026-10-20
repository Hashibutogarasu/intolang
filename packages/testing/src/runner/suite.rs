use ast::Program;
use errors::{Errors, Result};
use intovm::VmOptions;
use parser::PathLexeme;
use std::path::Path;

use super::builtins::{ASSERTION, BUILTIN_ALIAS, TEST_MACRO};
use super::discover::{find_tests, test_attribute};
use super::summary::TestSummary;
use crate::eval::run_message;

/// Run all `#[test]` functions in one Into source file. Each test body is
/// lowered as a program of its own and run in the Into VM.
pub fn run_tests(source: &str, path: &Path, packages_dir: &Path) -> Result<TestSummary> {
    let path_text = PathLexeme::from(path).to_string();
    let program = source
        .parse::<Program>()
        .map_err(|error| error.bound_to(&path_text))?;
    let imported = moduler::resolve(&program, packages_dir)?;
    let package =
        moduler::find_by_alias(packages_dir, BUILTIN_ALIAS)?.ok_or(Errors::BuiltinNotBundled)?;
    if !imported.contains(ASSERTION.function) {
        return Err(Errors::TestMustImportAssertion {
            path: path_text,
            function: ASSERTION.function.to_string(),
        }
        .into());
    }
    if !imported.contains(TEST_MACRO.function) {
        return Err(Errors::TestMustImportMacro { path: path_text }.into());
    }
    let attribute = test_attribute(&package)?;

    let mut summary = TestSummary::default();
    for test in find_tests(&program, &attribute) {
        let outcome = backend::lower_test(source, path, Some(packages_dir), &test.name)
            .and_then(|message| run_message(&message, VmOptions::default()));
        match outcome {
            Ok(_) => {
                summary.passed += 1;
                progress::report(&format!("test {} ... ok", test.name));
            }
            Err(error) => {
                summary.failed += 1;
                progress::report(&format!("test {} ... FAILED\n{error:#}", test.name));
            }
        }
    }
    if summary.passed + summary.failed == 0 {
        return Err(Errors::NoTestFunctions { path: path_text }.into());
    }
    Ok(summary)
}
