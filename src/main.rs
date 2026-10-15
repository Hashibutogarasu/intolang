use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "intolang", about = "Build an Into package")]
struct Args {
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
    let args = Args::parse();
    let manifest = if args.package.is_dir() { args.package.join("into-package.json") } else { args.package };
    let manifest = std::fs::canonicalize(&manifest)
        .with_context(|| format!("Could not find package manifest: {}", manifest.display()))?;
    let package_dir = manifest.parent().context("Manifest has no parent directory")?;
    let package: Package = serde_json::from_slice(&std::fs::read(&manifest)?)
        .with_context(|| format!("Invalid package manifest: {}", manifest.display()))?;
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

    let embedded_packages = compiler::builtin_packages_dir();
    let message = backend::lower_file(&source, &entry, Some(&embedded_packages))
        .context("Could not compile Into entry point")?;
    std::fs::create_dir_all(&output_dir)?;
    compiler::compile(&message, &output).context("Compilation failed")?;
    println!("Built {}", output.display());
    Ok(())
}

