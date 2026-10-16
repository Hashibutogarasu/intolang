use anyhow::{Context, Result, bail};
use clap::{ArgMatches, Args as ClapArgs, Command as ClapCommand, FromArgMatches};
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::commands::command::Command;

#[derive(Deserialize)]
struct PackageManifest {
    alias: String,
    version: String,
}

#[derive(ClapArgs)]
struct InstallArgs {
    /// Package directory to install into the current project
    source: PathBuf,
}

pub(super) struct Install;

impl Command for Install {
    fn command(&self) -> ClapCommand {
        InstallArgs::augment_args(
            ClapCommand::new("install").about("Install a package into the current project"),
        )
    }

    fn depends(&self) -> Vec<Box<dyn Command>> {
        Vec::new()
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let args = InstallArgs::from_arg_matches(matches)?;
        install(args.source)
    }
}

fn install(source: PathBuf) -> Result<()> {
    let source = std::fs::canonicalize(&source)
        .with_context(|| format!("Could not find package directory: {}", source.display()))?;
    let package_manifest = source.join("into-package.json");
    let package: PackageManifest =
        serde_json::from_slice(&std::fs::read(&package_manifest).with_context(|| {
            format!(
                "Could not read package manifest: {}",
                package_manifest.display()
            )
        })?)
        .with_context(|| format!("Invalid package manifest: {}", package_manifest.display()))?;
    if !valid_alias(&package.alias) {
        bail!("Invalid package alias `{}`", package.alias);
    }

    let project = std::env::current_dir()?;
    let project_manifest = project.join("into-package.json");
    let mut project_json: Value =
        serde_json::from_slice(&std::fs::read(&project_manifest).with_context(|| {
            format!(
                "Could not find project manifest: {}",
                project_manifest.display()
            )
        })?)
        .with_context(|| format!("Invalid project manifest: {}", project_manifest.display()))?;
    let dependencies = project_json
        .get_mut("packages")
        .and_then(Value::as_object_mut)
        .context("Project manifest must contain a `packages` object")?;
    dependencies.insert(
        package.alias.clone(),
        Value::String(package.version.clone()),
    );

    let package_root = project.join(".into/packages");
    std::fs::create_dir_all(&package_root)?;
    let destination = package_root.join(&package.alias);
    let staging = package_root.join(format!(
        ".install-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    copy_package(&source, &staging)?;

    let manifest_staging = project_manifest.with_extension("json.install-tmp");
    let mut manifest_bytes = serde_json::to_vec_pretty(&project_json)?;
    manifest_bytes.push(b'\n');
    std::fs::write(&manifest_staging, manifest_bytes)?;
    if destination.exists() {
        std::fs::remove_dir_all(&destination)
            .with_context(|| format!("Could not replace installed package `{}`", package.alias))?;
    }
    std::fs::rename(&staging, &destination)?;
    std::fs::rename(&manifest_staging, &project_manifest)?;
    println!("Installed {} {}", package.alias, package.version);
    Ok(())
}

fn valid_alias(alias: &str) -> bool {
    !alias.is_empty()
        && alias != "."
        && alias != ".."
        && alias
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn copy_package(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".git" || name == "dist" {
            continue;
        }
        let target = destination.join(name);
        if entry.file_type()?.is_dir() {
            copy_package(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
