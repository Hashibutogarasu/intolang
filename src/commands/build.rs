use clap::{ArgMatches, Args, Command as ClapCommand};
use errors::{Context, Errors, Result};
use progress::Progress;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::commands::command::{Command, arguments};
use crate::commands::inex;
use crate::commands::target::{InexTarget, NativeTarget, Project, Target};

#[derive(Deserialize)]
struct Package {
    name: String,
    #[serde(default)]
    alias: Option<String>,
    #[serde(rename = "builtConfig", default)]
    built_config: BuildConfig,
    runs: Option<String>,
    library: Option<String>,
    #[serde(default)]
    packages: HashMap<String, String>,
}

#[derive(Deserialize)]
struct BuildConfig {
    #[serde(default = "default_dist")]
    dist: String,
}

fn default_dist() -> String {
    "./dist".into()
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            dist: default_dist(),
        }
    }
}

pub(super) fn run(
    package_path: PathBuf,
    test_mode: bool,
    run_after_build: bool,
    vm_output: bool,
) -> Result<()> {
    if inex::is_inex(&package_path) {
        return inex::run_file(&package_path);
    }
    let manifest = if package_path.is_dir() {
        package_path.join("into-package.json")
    } else {
        package_path
    };
    let manifest =
        std::fs::canonicalize(&manifest).map_err(|error| Errors::PackageManifestNotFound {
            path: manifest.display().to_string(),
            reason: error.to_string(),
        })?;
    let package_dir = manifest.parent().ok_or(Errors::ManifestNoParent)?;
    let package: Package = serde_json::from_slice(
        &std::fs::read(&manifest).map_err(|error| Errors::read(&manifest, error))?,
    )
    .map_err(|error| Errors::PackageManifestInvalid {
        path: manifest.display().to_string(),
        reason: error.to_string(),
    })?;
    if package.runs.is_some() == package.library.is_some() {
        return Err(Errors::ManifestRunsOrLibrary.into());
    }
    let embedded_packages = compiler::builtin_packages_dir();
    if test_mode {
        return run_tests(package_dir, &embedded_packages);
    }
    if vm_output && package.library.is_some() {
        return Err(Errors::VmLibraryUnsupported.into());
    }
    let target: Box<dyn Target> = if vm_output {
        Box::new(InexTarget)
    } else {
        Box::new(NativeTarget)
    };
    let steps = if package.library.is_some() {
        2
    } else {
        1 + target.steps()
    };
    let progress = Progress::start("Preparing the packages", steps);
    let package_environment =
        prepare_package_environment(package_dir, &embedded_packages, &package.packages)?;

    if let Some(library) = package.library.as_deref() {
        return build_library(
            package_dir,
            &package,
            &library,
            package_environment.path(),
            progress,
        );
    }
    let entry = package_dir.join(package.runs.expect("runs/library validation above"));
    let source = std::fs::read_to_string(&entry).map_err(|error| Errors::read(&entry, error))?;
    let output_dir = package_dir
        .join(&package.built_config.dist)
        .join(std::env::consts::OS)
        .join(std::env::consts::ARCH);
    let executable = package
        .alias
        .as_deref()
        .unwrap_or(&package.name)
        .replace(' ', "-");
    let output = target.output(&output_dir, &executable);
    if output.exists() {
        std::fs::remove_file(&output)?;
    }
    progress.step("Lowering the Into sources");
    let message = backend::lower_file(&source, &entry, Some(package_environment.path()))
        .context("Could not compile Into entry point")?;
    std::fs::create_dir_all(&output_dir)?;
    let project = Project {
        directory: package_dir,
        packages: package_environment.path(),
    };
    target.build(&project, message, &output, &progress)?;
    progress.finish(&progress::built(&executable, &output));
    if run_after_build {
        target.run(&project, &output)?;
    }
    Ok(())
}

#[derive(Args)]
struct BuildArgs {
    #[arg(default_value = ".")]
    package: PathBuf,
    #[arg(long = "test")]
    test: bool,
    /// Write an .inex file for the Into VM instead of a native executable
    #[arg(long = "vm")]
    vm: bool,
}

pub(super) struct Build;

impl Command for Build {
    fn command(&self) -> ClapCommand {
        BuildArgs::augment_args(ClapCommand::new("build").about("Build an Into package"))
    }

    fn depends(&self) -> Vec<Box<dyn Command>> {
        Vec::new()
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let args = arguments::<BuildArgs>(matches)?;
        run(args.package, args.test, false, args.vm)
    }
}

#[derive(Args)]
struct RunArgs {
    /// Package directory, its into-package.json or an .inex file
    #[arg(default_value = ".")]
    package: PathBuf,
    /// Build an .inex file and run it in the Into VM
    #[arg(long = "vm")]
    vm: bool,
}

pub(super) struct Run;

impl Command for Run {
    fn command(&self) -> ClapCommand {
        RunArgs::augment_args(ClapCommand::new("run").about("Build and run an Into package"))
    }

    fn depends(&self) -> Vec<Box<dyn Command>> {
        Vec::new()
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let args = arguments::<RunArgs>(matches)?;
        run(args.package, false, true, args.vm)
    }
}

fn build_library(
    package_dir: &Path,
    package: &Package,
    library: &str,
    package_environment: &Path,
    progress: Progress,
) -> Result<()> {
    progress.step("Lowering the library");
    let required_entry = package_dir.join("src/main/lib.into");
    let library_entry = package_dir.join(library);
    if library_entry != required_entry || !required_entry.is_file() {
        return Err(Errors::LibraryEntryInvalid.into());
    }
    let source = std::fs::read_to_string(&required_entry)
        .map_err(|error| Errors::read(&required_entry, error))?;
    let functions = backend::lower_library(&source, &required_entry, package_environment)
        .context("Could not compile Into library entry point")?;
    let output_dir = package_dir
        .join(&package.built_config.dist)
        .join(std::env::consts::OS)
        .join(std::env::consts::ARCH);
    let basename = package
        .alias
        .as_deref()
        .unwrap_or(&package.name)
        .replace(' ', "-");
    #[cfg(target_os = "linux")]
    let filename = format!("lib{basename}.so");
    #[cfg(target_os = "macos")]
    let filename = format!("lib{basename}.dylib");
    #[cfg(target_os = "windows")]
    let filename = format!("{basename}.dll");
    let output = output_dir.join(filename);
    std::fs::create_dir_all(&output_dir)?;
    progress.step("Compiling the library");
    compiler::compile_library(&functions, &output).context("Native library compilation failed")?;
    progress.finish(&progress::built(&basename, &output));
    Ok(())
}

struct PackageEnvironment(PathBuf);

impl PackageEnvironment {
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for PackageEnvironment {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn prepare_package_environment(
    project: &Path,
    embedded: &Path,
    dependencies: &HashMap<String, String>,
) -> Result<PackageEnvironment> {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let root =
        std::env::temp_dir().join(format!("intolang-packages-{}-{suffix}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    for entry in std::fs::read_dir(embedded)? {
        let entry = entry?;
        if entry.path().is_dir() {
            let target = std::fs::canonicalize(entry.path())?;
            moduler::link_directory(&target, &root.join(entry.file_name()))?;
        }
    }

    let installed = project.join(".into/packages");
    if installed.is_dir() {
        for entry in std::fs::read_dir(&installed)? {
            let entry = entry?;
            if !entry.path().is_dir() {
                continue;
            }
            let manifest_path = entry.path().join("into-package.json");
            let Ok(contents) = std::fs::read(&manifest_path) else {
                continue;
            };
            let Ok(manifest): std::result::Result<serde_json::Value, _> =
                serde_json::from_slice(&contents)
            else {
                continue;
            };
            let Some(alias) = manifest.get("alias").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let Some(declared) = dependencies
                .get(alias)
                .map(|version| version.trim())
                .filter(|version| !version.is_empty())
            else {
                continue;
            };
            if !valid_alias(alias) {
                return Err(Errors::InstalledAliasInvalid {
                    alias: alias.to_string(),
                }
                .into());
            }
            let installed_version = manifest
                .get("version")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if installed_version != declared {
                return Err(Errors::InstalledVersionMismatch {
                    alias: alias.to_string(),
                    declared: declared.to_string(),
                    installed: installed_version.to_string(),
                }
                .into());
            }
            let target = std::fs::canonicalize(entry.path())?;
            moduler::link_directory(&target, &root.join(alias))?;
        }
    }
    Ok(PackageEnvironment(root))
}

fn valid_alias(alias: &str) -> bool {
    !alias.is_empty()
        && alias != "."
        && alias != ".."
        && alias
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn run_tests(package_dir: &Path, embedded_packages: &Path) -> Result<()> {
    let test_root = package_dir.join("src/tests");
    let mut test_files = Vec::new();
    collect_into_files(&test_root, &mut test_files)?;
    if test_files.is_empty() {
        return Err(Errors::NoTestFiles {
            path: test_root.display().to_string(),
        }
        .into());
    }
    let mut passed = 0;
    let mut failed = 0;
    for file in test_files {
        let source = std::fs::read_to_string(&file).map_err(|error| Errors::read(&file, error))?;
        let result = testing::run_tests(&source, &file, embedded_packages)
            .with_context(|| format!("Failed running tests in {}", file.display()))?;
        passed += result.passed;
        failed += result.failed;
    }
    progress::report(&format!(
        "test result: {}. {passed} passed; {failed} failed",
        if failed == 0 { "ok" } else { "FAILED" }
    ));
    if failed > 0 {
        return Err(Errors::TestsFailed { count: failed }.into());
    }
    Ok(())
}

fn collect_into_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_into_files(&path, files)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "into")
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(())
}
