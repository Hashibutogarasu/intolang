use clap::{ArgMatches, Args as ClapArgs, Command as ClapCommand};
use errors::{Context, Errors, Result};
use std::path::{Path, PathBuf};

use crate::commands::command::{Command, arguments};

#[derive(ClapArgs)]
struct NewArgs {
    path: PathBuf,
    #[arg(long = "licence", default_value = "MIT")]
    licence: String,
    #[arg(long = "version", default_value = "0.1.0")]
    version: String,
    #[arg(long = "alias")]
    alias: Option<String>,
}

pub(super) struct New;

impl Command for New {
    fn command(&self) -> ClapCommand {
        NewArgs::augment_args(
            ClapCommand::new("new").about("Create an Into project from the hello_world template"),
        )
    }

    fn depends(&self) -> Vec<Box<dyn Command>> {
        Vec::new()
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let args = arguments::<NewArgs>(matches)?;
        create(args)
    }
}

fn create(args: NewArgs) -> Result<()> {
    let project_name = args
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(Errors::ProjectNameInvalid)?;
    if args.path == Path::new(".") || args.path == Path::new("..") {
        return Err(Errors::ProjectPathNotNew.into());
    }
    if args.path.exists() {
        return Err(Errors::ProjectExists {
            path: args.path.display().to_string(),
        }
        .into());
    }

    if let Some(parent) = args.path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| Errors::file_action("create directory", parent, error))?;
    }
    copy_directory(&compiler::hello_world_template_dir(), &args.path).with_context(|| {
        format!(
            "Could not copy project template into {}",
            args.path.display()
        )
    })?;

    let manifest_path = args.path.join("into-package.json");
    let mut manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    manifest["name"] = serde_json::Value::String(project_name.to_owned());
    manifest["alias"] =
        serde_json::Value::String(args.alias.unwrap_or_else(|| project_name.to_owned()));
    manifest["version"] = serde_json::Value::String(args.version);
    manifest["LICENCE"] = serde_json::Value::String(args.licence);
    std::fs::write(
        &manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    progress::report(&format!("Created project {}", args.path.display()));
    Ok(())
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
