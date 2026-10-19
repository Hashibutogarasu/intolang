use anyhow::{Context, Result};
use intovm::Inex;
use std::path::{Path, PathBuf};

use crate::commands::inex;

/// What a build knows about the package it builds.
pub(super) struct Project<'a> {
    pub directory: &'a Path,
    pub packages: &'a Path,
}

/// The binary a build writes from the lowered message of a package: how it is
/// named, how it is made and how it runs.
pub(super) trait Target {
    fn output(&self, directory: &Path, executable: &str) -> PathBuf;

    fn build(&self, project: &Project, message: String, output: &Path) -> Result<()>;

    fn run(&self, project: &Project, output: &Path) -> Result<()>;
}

/// A native executable, compiled with LLVM and linked.
pub(super) struct NativeTarget;

impl Target for NativeTarget {
    fn output(&self, directory: &Path, executable: &str) -> PathBuf {
        #[cfg(windows)]
        let executable = format!("{executable}.exe");
        directory.join(executable)
    }

    fn build(&self, project: &Project, message: String, output: &Path) -> Result<()> {
        let carried = Inex {
            generator: format!("into {}", env!("CARGO_PKG_VERSION")),
            bundle: inex::resources(project.directory, &message)?,
        };
        let assets = carried
            .bundle
            .entries()
            .next()
            .is_some()
            .then(|| Vec::from(&carried));
        compiler::compile_with_assets(&message, output, project.packages, assets.as_deref())
            .context("Compilation failed")
    }

    fn run(&self, project: &Project, output: &Path) -> Result<()> {
        let status = std::process::Command::new(output)
            .current_dir(project.directory)
            .status()
            .with_context(|| format!("Could not run compiled executable {}", output.display()))?;
        if !status.success() {
            anyhow::bail!("Program exited with status {status}");
        }
        Ok(())
    }
}

/// An `.inex` file that the Into VM runs.
pub(super) struct InexTarget;

impl Target for InexTarget {
    fn output(&self, directory: &Path, executable: &str) -> PathBuf {
        directory.join(format!("{executable}.{}", inex::EXTENSION))
    }

    fn build(&self, project: &Project, message: String, output: &Path) -> Result<()> {
        let program = inex::assemble(
            project.directory,
            project.packages,
            message,
            format!("into {}", env!("CARGO_PKG_VERSION")),
        )?;
        inex::write(&program, output)
    }

    fn run(&self, project: &Project, output: &Path) -> Result<()> {
        std::env::set_current_dir(project.directory).with_context(|| {
            format!(
                "Could not enter the package {}",
                project.directory.display()
            )
        })?;
        inex::run_file(&output.to_path_buf())
    }
}
