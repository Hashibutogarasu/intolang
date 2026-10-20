use errors::{Errors, Result};

use super::failure::{tool_failure, tool_not_run};

/// Run `pkg-config <flag> <dependencies...>` and split its output into words.
pub fn pkg_config_flags(dependencies: &[String], flag: &str) -> Result<Vec<String>> {
    if dependencies.is_empty() {
        return Ok(Vec::new());
    }
    let output = std::process::Command::new("pkg-config")
        .arg(flag)
        .args(dependencies)
        .output()
        .map_err(|error| tool_not_run("pkg-config", &error))?;
    if !output.status.success() {
        return Err(tool_failure(
            Errors::PkgConfigFailed {
                dependencies: format!("{dependencies:?}"),
            },
            "pkg-config",
            &output.stderr,
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .map(str::to_owned)
        .collect())
}
