use anyhow::{Context, Result};
use clap::{ArgMatches, Args as ClapArgs, Command as ClapCommand, FromArgMatches};
use moduler::{Project, find_by_alias, is_builtin};
use std::path::PathBuf;

use crate::commands::command::Command;

#[derive(ClapArgs)]
struct InstallArgs {
    /// Package directory to install into the current project; without it, the
    /// packages recorded in into-package-lock.json are restored. `builtin` (or
    /// `intolang`) references the package embedded in the compiler
    source: Option<PathBuf>,
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
        let project = Project::new(std::env::current_dir()?);
        let embedded = compiler::builtin_packages_dir();
        match args.source {
            Some(source) => {
                let source = match source.to_str() {
                    Some(name) if is_builtin(name) && !source.exists() => {
                        find_by_alias(&embedded, moduler::BUILTIN_ALIAS)?
                            .context("The compiler does not embed the builtin package")?
                    }
                    _ => source,
                };
                let package = project.install(&source, &embedded)?;
                progress::report(&format!("Installed {} {}", package.alias, package.version));
            }
            None => {
                for restoration in project.restore(&embedded)? {
                    let status = if restoration.installed {
                        "Installed"
                    } else {
                        "Up to date"
                    };
                    progress::report(&format!(
                        "{status} {} {}",
                        restoration.package.alias, restoration.package.version
                    ));
                }
            }
        }
        Ok(())
    }
}
