use anyhow::{Result, anyhow, bail};
use ast::{Program, SyntaxError};
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
    let program =
        source
            .parse::<Program>()
            .map_err(|error| match error.downcast_ref::<SyntaxError>() {
                Some(syntax) => anyhow!("{}", syntax.backtrace(&path_text)),
                None => error,
            })?;
    let imported = moduler::resolve(&program, packages_dir)?;
    let package = moduler::find_by_alias(packages_dir, BUILTIN_ALIAS)?
        .ok_or_else(|| anyhow!("Builtin package is not bundled"))?;
    if !imported.contains(ASSERTION.function) {
        bail!(
            "{path_text}: test module must import `{}`",
            ASSERTION.function
        );
    }
    if !imported.contains(TEST_MACRO.function) {
        bail!("{path_text}: test module must import the test macro");
    }
    let attribute = test_attribute(&package)?;

    let mut summary = TestSummary::default();
    for test in find_tests(&program, &attribute) {
        let outcome = backend::lower_test(source, path, Some(packages_dir), &test.name)
            .and_then(|message| run_message(&message));
        match outcome {
            Ok(_) => {
                summary.passed += 1;
                println!("test {} ... ok", test.name);
            }
            Err(error) => {
                summary.failed += 1;
                eprintln!("test {} ... FAILED\n{error:#}", test.name);
            }
        }
    }
    if summary.passed + summary.failed == 0 {
        bail!("{path_text}: no #[test] functions found");
    }
    Ok(summary)
}
