use errors::{Context, Errors, Result};

use super::execution::Execution;
use super::workspace::Workspace;

/// Compile Into source against the bundled builtin packages, run the
/// executable and capture what it printed.
pub fn eval_run(source: &str) -> Result<Execution> {
    let packages = compiler::builtin_packages_dir();
    let message = backend::lower_with_packages(source, Some(&packages))?;
    let workspace = Workspace::with_packages(&[])?;
    let executable = workspace.path().join("program");
    compiler::compile_with_packages(&message, &executable, &packages)
        .context("Compilation failed")?;
    let output = std::process::Command::new(&executable)
        .current_dir(workspace.path())
        .output()
        .map_err(|error| Errors::ToolNotRun {
            tool: executable.display().to_string(),
            reason: error.to_string(),
        })?;
    Ok(Execution {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
        code: output.status.code(),
    })
}
