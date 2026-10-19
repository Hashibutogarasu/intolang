use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use types::Language;

use super::compile::toolchain;
use super::pkg_config::pkg_config_flags;

/// A native source to compile into a shared library.
#[derive(Debug, Clone)]
pub struct SharedSource {
    pub path: PathBuf,
    pub language: Language,
    /// The `pkg-config` libraries the source needs.
    pub dependencies: Vec<String>,
}

/// Compile `sources` into the shared library `output`, which a program can
/// load while it runs.
pub fn compile_shared_library(sources: &[SharedSource], output: &Path) -> Result<()> {
    let uses_cxx = sources
        .iter()
        .any(|source| source.language == Language::Cpp);
    let mut objects = Vec::new();
    let mut dependencies = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        for library in &source.dependencies {
            if !dependencies.contains(library) {
                dependencies.push(library.clone());
            }
        }
        let object = output.with_file_name(format!("native-{index}.o"));
        let (variable, default) = toolchain(source.language);
        let command = std::env::var(variable).unwrap_or_else(|_| default.into());
        let result = std::process::Command::new(&command)
            .arg("-fPIC")
            .arg("-c")
            .args(pkg_config_flags(&source.dependencies, "--cflags")?)
            .arg(&source.path)
            .arg("-o")
            .arg(&object)
            .output()
            .with_context(|| {
                format!(
                    "native call requires a compiler: could not run `{command}` for {}",
                    source.path.display()
                )
            })?;
        if !result.status.success() {
            bail!(
                "Native source compilation failed for {}:\n{}",
                source.path.display(),
                String::from_utf8_lossy(&result.stderr)
            );
        }
        objects.push(object);
    }
    let (variable, default) = toolchain(if uses_cxx { Language::Cpp } else { Language::C });
    let command = std::env::var(variable).unwrap_or_else(|_| default.into());
    let result = std::process::Command::new(&command)
        .arg("-shared")
        .args(&objects)
        .arg("-o")
        .arg(output)
        .args(pkg_config_flags(&dependencies, "--libs")?)
        .output()
        .with_context(|| format!("native call requires a compiler: could not run `{command}`"))?;
    for object in &objects {
        std::fs::remove_file(object).ok();
    }
    if !result.status.success() {
        bail!(
            "Linking the native library failed:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    Ok(())
}
