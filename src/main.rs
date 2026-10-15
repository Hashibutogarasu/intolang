use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::{ffi::OsString, path::{Path, PathBuf}};

#[derive(Parser)]
#[command(name = "intolang", about = "Build an Into package")]
struct Args {
    #[arg(long = "test")]
    test_mode: bool,
    #[arg(long = "format")]
    format_mode: bool,
    /// Maximum directory depth to search in format mode (0 means this directory only)
    #[arg(long = "depth", requires = "format_mode")]
    format_depth: Option<usize>,
    /// Package directory or path to its into-package.json
    package: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    #[serde(default)]
    alias: Option<String>,
    #[serde(rename = "builtConfig", default)]
    built_config: BuildConfig,
    runs: String,
}

#[derive(Deserialize)]
struct BuildConfig { #[serde(default = "default_dist")] dist: String }
impl Default for BuildConfig { fn default() -> Self { Self { dist: default_dist() } } }
fn default_dist() -> String { "./dist".into() }

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    run().map_err(|error| color_eyre::eyre::eyre!("{error:#}"))
}

fn run() -> Result<()> {
    let args = parse_args();
    if args.format_mode {
        let mut files = Vec::new();
        if args.package.is_dir() {
            collect_into_files_at_depth(&args.package, &mut files, args.format_depth, 0)?;
            files.sort();
            if files.is_empty() {
                anyhow::bail!("No .into files found under {}", args.package.display());
            }
        } else {
            if args.package.extension().is_none_or(|extension| extension != "into") {
                anyhow::bail!("Format mode expects an .into file or directory path");
            }
            files.push(args.package);
        }

        for file in files {
            let source = std::fs::read_to_string(&file)
                .with_context(|| format!("Could not read Into source: {}", file.display()))?;
            let formatted = backend::format_source(&source);
            std::fs::write(&file, formatted)
                .with_context(|| format!("Could not write formatted Into source: {}", file.display()))?;
            println!("Formatted {}", file.display());
        }
        return Ok(());
    }
    let manifest = if args.package.is_dir() { args.package.join("into-package.json") } else { args.package };
    let manifest = std::fs::canonicalize(&manifest)
        .with_context(|| format!("Could not find package manifest: {}", manifest.display()))?;
    let package_dir = manifest.parent().context("Manifest has no parent directory")?;
    let package: Package = serde_json::from_slice(&std::fs::read(&manifest)?)
        .with_context(|| format!("Invalid package manifest: {}", manifest.display()))?;
    let embedded_packages = compiler::builtin_packages_dir();
    if args.test_mode {
        let test_root = package_dir.join("src/tests");
        let mut test_files = Vec::new();
        collect_into_files(&test_root, &mut test_files)?;
        if test_files.is_empty() { anyhow::bail!("No Into test files found under {}", test_root.display()); }
        let mut passed = 0;
        let mut failed = 0;
        for file in test_files {
            let source = std::fs::read_to_string(&file)?;
            let result = backend::run_tests(&source, &file, &embedded_packages)
                .with_context(|| format!("Failed running tests in {}", file.display()))?;
            passed += result.passed;
            failed += result.failed;
        }
        println!("test result: {}. {passed} passed; {failed} failed", if failed == 0 { "ok" } else { "FAILED" });
        if failed > 0 { anyhow::bail!("{failed} Into test(s) failed"); }
        return Ok(());
    }
    let entry = package_dir.join(&package.runs);
    let source = std::fs::read_to_string(&entry)
        .with_context(|| format!("Could not read package entry point: {}", entry.display()))?;
    let output_dir = package_dir.join(&package.built_config.dist)
        .join(std::env::consts::OS).join(std::env::consts::ARCH);
    let executable = package.alias.as_deref().unwrap_or(&package.name).replace(' ', "-");
    #[cfg(windows)]
    let executable = format!("{executable}.exe");
    let output = output_dir.join(executable);
    if output.exists() { std::fs::remove_file(&output)?; }

    let message = backend::lower_file(&source, &entry, Some(&embedded_packages))
        .context("Could not compile Into entry point")?;
    std::fs::create_dir_all(&output_dir)?;
    compiler::compile(&message, &output).context("Compilation failed")?;
    println!("Built {}", output.display());
    Ok(())
}

fn parse_args() -> Args {
    let argv = std::env::args_os().map(|arg| {
        match arg.to_str() {
            Some("-test") => OsString::from("--test"),
            Some("-format") => OsString::from("--format"),
            Some("-depth") => OsString::from("--depth"),
            _ => arg,
        }
    });
    Args::parse_from(argv)
}

fn collect_into_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    collect_into_files_at_depth(root, files, None, 0)
}

fn collect_into_files_at_depth(
    root: &Path,
    files: &mut Vec<PathBuf>,
    max_depth: Option<usize>,
    depth: usize,
) -> Result<()> {
    if !root.exists() { return Ok(()); }
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() && max_depth.is_none_or(|limit| depth < limit) {
            collect_into_files_at_depth(&path, files, max_depth, depth + 1)?;
        }
        else if path.extension().is_some_and(|extension| extension == "into") { files.push(path); }
    }
    Ok(())
}

