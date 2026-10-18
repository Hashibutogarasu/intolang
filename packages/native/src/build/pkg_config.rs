use anyhow::{Context, Result, bail};

/// Run `pkg-config <flag> <dependencies...>` and split its output into words.
pub fn pkg_config_flags(dependencies: &[String], flag: &str) -> Result<Vec<String>> {
    if dependencies.is_empty() {
        return Ok(Vec::new());
    }
    let output = std::process::Command::new("pkg-config")
        .arg(flag)
        .args(dependencies)
        .output()
        .context("Could not run pkg-config for declared native dependencies")?;
    if !output.status.success() {
        bail!(
            "pkg-config failed for native dependencies {:?}: {}",
            dependencies,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .map(str::to_owned)
        .collect())
}
