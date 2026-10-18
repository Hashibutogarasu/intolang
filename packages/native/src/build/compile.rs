use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use types::Language;

use super::manifest::native_dependencies;
use super::pkg_config::pkg_config_flags;
use super::sources::collect_sources;

/// The result of compiling the native sources a message links against.
#[derive(Debug, Default)]
pub struct NativeBuild {
    pub objects: Vec<PathBuf>,
    pub uses_cxx: bool,
    pub libraries: Vec<String>,
    pub library_paths: Vec<String>,
}

/// The environment variable that overrides, and the default for, the compiler
/// of a source language.
fn toolchain(language: Language) -> (&'static str, &'static str) {
    match language {
        Language::C => ("CC", "cc"),
        Language::Cpp => ("CXX", "c++"),
    }
}

/// Compile every native source `message` links against into an object file
/// next to `output`, collecting the libraries their packages depend on.
pub fn compile_sources(message: &str, output: &Path, packages_dir: &Path) -> Result<NativeBuild> {
    let mut build = NativeBuild::default();
    for (index, source) in collect_sources(message, packages_dir)?.iter().enumerate() {
        build.uses_cxx |= source.language == Language::Cpp;
        let dependencies = native_dependencies(&source.package, &source.path)?;
        for library in &dependencies {
            let path = pkg_config_flags(std::slice::from_ref(library), "--variable=libdir")?
                .into_iter()
                .next()
                .with_context(|| {
                    format!("pkg-config returned no library directory for `{library}`")
                })?;
            if !build.library_paths.contains(&path) {
                build.library_paths.push(path);
            }
        }
        for flag in pkg_config_flags(&dependencies, "--libs")? {
            if let Some(name) = flag.strip_prefix("-l") {
                if !name.is_empty() && !build.libraries.iter().any(|library| library == name) {
                    build.libraries.push(name.to_owned());
                }
            } else if let Some(path) = flag.strip_prefix("-L") {
                if !build.library_paths.iter().any(|existing| existing == path) {
                    build.library_paths.push(path.to_owned());
                }
            }
        }
        let object =
            output.with_file_name(format!(".intolang-native-{}-{index}.o", std::process::id()));
        let (variable, default) = toolchain(source.language);
        let command = std::env::var(variable).unwrap_or_else(|_| default.into());
        let result = std::process::Command::new(command)
            .arg("-fPIC")
            .arg("-c")
            .args(pkg_config_flags(&dependencies, "--cflags")?)
            .arg(&source.path)
            .arg("-o")
            .arg(&object)
            .output()
            .with_context(|| {
                format!(
                    "Could not compile native module source {}",
                    source.path.display()
                )
            })?;
        if !result.status.success() {
            bail!(
                "Native module compilation failed for {}:\n{}",
                source.path.display(),
                String::from_utf8_lossy(&result.stderr)
            );
        }
        build.objects.push(object);
    }
    Ok(build)
}
