use anyhow::{Context, Result};
use clap::Parser;
use std::{ffi::OsString, path::{Path, PathBuf}};

#[derive(Parser)]
#[command(name = "intolang new", about = "Create an Into project from the hello_world template")]
struct Args {
    path: PathBuf,
    #[arg(long = "licence", default_value = "MIT")]
    licence: String,
    #[arg(long = "version", default_value = "0.1.0")]
    version: String,
    #[arg(long = "alias")]
    alias: Option<String>,
}

pub(super) fn run() -> Result<()> {
    let args = parse_args();
    let project_name = args.path.file_name()
        .and_then(|name| name.to_str())
        .context("Project path must end with a valid project name")?;
    if args.path == Path::new(".") || args.path == Path::new("..") {
        anyhow::bail!("Project path must name a new directory");
    }
    if args.path.exists() {
        anyhow::bail!("Project destination already exists: {}", args.path.display());
    }

    if let Some(parent) = args.path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Could not create parent directory {}", parent.display()))?;
    }
    copy_directory(&compiler::hello_world_template_dir(), &args.path)
        .with_context(|| format!("Could not copy project template into {}", args.path.display()))?;

    let manifest_path = args.path.join("into-package.json");
    let mut manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    manifest["name"] = serde_json::Value::String(project_name.to_owned());
    manifest["alias"] = serde_json::Value::String(args.alias.unwrap_or_else(|| project_name.to_owned()));
    manifest["version"] = serde_json::Value::String(args.version);
    manifest["LICENCE"] = serde_json::Value::String(args.licence);
    std::fs::write(&manifest_path, format!("{}\n", serde_json::to_string_pretty(&manifest)?))?;
    println!("Created project {}", args.path.display());
    Ok(())
}

fn parse_args() -> Args {
    let argv = std::env::args_os().skip(2).map(|arg| match arg.to_str() {
        Some("-licence") => OsString::from("--licence"),
        Some("-version") => OsString::from("--version"),
        Some("-alias") => OsString::from("--alias"),
        _ => arg,
    });
    Args::parse_from(std::iter::once(OsString::from("intolang new")).chain(argv))
}

fn copy_directory(source: &Path, target: &Path) -> Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let destination = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory(&entry.path(), &destination)?;
        } else {
            std::fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}
