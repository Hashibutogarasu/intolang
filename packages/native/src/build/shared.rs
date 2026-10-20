use errors::{Errors, Result};
use std::path::{Path, PathBuf};
use types::Language;

use super::compile::toolchain;
use super::failure::{tool_failure, tool_not_run};
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
            .map_err(|error| tool_not_run(&command, &error))?;
        if !result.status.success() {
            return Err(tool_failure(
                Errors::NativeSourceCompilationFailed {
                    path: source.path.display().to_string(),
                },
                &command,
                &result.stderr,
            ));
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
        .map_err(|error| tool_not_run(&command, &error))?;
    for object in &objects {
        std::fs::remove_file(object).ok();
    }
    if !result.status.success() {
        return Err(tool_failure(
            Errors::NativeLibraryLinkFailed,
            &command,
            &result.stderr,
        ));
    }
    Ok(())
}
