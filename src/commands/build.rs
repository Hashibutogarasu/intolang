use anyhow::{Context, Result};
use clap::{ArgMatches, Args, Command as ClapCommand, FromArgMatches};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::commands::command::Command;

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

pub(super) fn run(package_path: PathBuf, test_mode: bool, run_after_build: bool) -> Result<()> {
    let manifest = if package_path.is_dir() {
        package_path.join("into-package.json")
    } else {
        package_path
    };
    let manifest = std::fs::canonicalize(&manifest)
        .with_context(|| format!("Could not find package manifest: {}", manifest.display()))?;
    let package_dir = manifest
        .parent()
        .context("Manifest has no parent directory")?;
    let package: Package = serde_json::from_slice(&std::fs::read(&manifest)?)
        .with_context(|| format!("Invalid package manifest: {}", manifest.display()))?;
    if package.runs.is_some() == package.library.is_some() {
        anyhow::bail!("Package manifest must specify exactly one of `runs` or `library`");
    }
    let embedded_packages = compiler::builtin_packages_dir();
    if test_mode {
        return run_tests(package_dir, &embedded_packages);
    }
    let package_environment =
        prepare_package_environment(package_dir, &embedded_packages, &package.packages)?;

    if let Some(library) = package.library.as_deref() {
        return build_library(package_dir, &package, &library, package_environment.path());
    }
    let entry = package_dir.join(package.runs.expect("runs/library validation above"));
    let source = std::fs::read_to_string(&entry)
        .with_context(|| format!("Could not read package entry point: {}", entry.display()))?;
    let output_dir = package_dir
        .join(&package.built_config.dist)
        .join(std::env::consts::OS)
        .join(std::env::consts::ARCH);
    let executable = package
        .alias
        .as_deref()
        .unwrap_or(&package.name)
        .replace(' ', "-");
    #[cfg(windows)]
    let executable = format!("{executable}.exe");
    let output = output_dir.join(executable);
    if output.exists() {
        std::fs::remove_file(&output)?;
    }

    let message = backend::lower_file(&source, &entry, Some(package_environment.path()))
        .context("Could not compile Into entry point")?;
    std::fs::create_dir_all(&output_dir)?;
    compiler::compile_with_packages(&message, &output, package_environment.path())
        .context("Compilation failed")?;
    println!("Built {}", output.display());
    if run_after_build {
        let status = std::process::Command::new(&output)
            .current_dir(package_dir)
            .status()
            .with_context(|| format!("Could not run compiled executable {}", output.display()))?;
        if !status.success() {
            anyhow::bail!("Program exited with status {status}");
        }
    }
    Ok(())
}

#[derive(Args)]
struct BuildArgs {
    #[arg(default_value = ".")]
    package: PathBuf,
    #[arg(long = "test")]
    test: bool,
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
        let args = BuildArgs::from_arg_matches(matches)?;
        run(args.package, args.test, false)
    }
}

#[derive(Args)]
struct RunArgs {
    #[arg(default_value = ".")]
    package: PathBuf,
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
        let args = RunArgs::from_arg_matches(matches)?;
        run(args.package, false, true)
    }
}

fn build_library(
    package_dir: &Path,
    package: &Package,
    library: &str,
    package_environment: &Path,
) -> Result<()> {
    let required_entry = package_dir.join("src/main/lib.into");
    let library_entry = package_dir.join(library);
    if library_entry != required_entry || !required_entry.is_file() {
        anyhow::bail!("Library packages require src/main/lib.into as their library entry point");
    }
    let source = std::fs::read_to_string(&required_entry).with_context(|| {
        format!(
            "Could not read library entry point: {}",
            required_entry.display()
        )
    })?;
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
    compiler::compile_library(&functions, &output).context("Native library compilation failed")?;
    println!("Built {}", output.display());
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
                anyhow::bail!("Invalid installed package alias `{alias}`");
            }
            let installed_version = manifest
                .get("version")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if installed_version != declared {
                anyhow::bail!(
                    "Package `{alias}` is declared as version {declared} but version {installed_version} is installed"
                );
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
        anyhow::bail!("No Into test files found under {}", test_root.display());
    }
    let mut passed = 0;
    let mut failed = 0;
    for file in test_files {
        let source = std::fs::read_to_string(&file)?;
        let result = testing::run_tests(&source, &file, embedded_packages)
            .with_context(|| format!("Failed running tests in {}", file.display()))?;
        passed += result.passed;
        failed += result.failed;
    }
    println!(
        "test result: {}. {passed} passed; {failed} failed",
        if failed == 0 { "ok" } else { "FAILED" }
    );
    if failed > 0 {
        anyhow::bail!("{failed} Into test(s) failed");
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
