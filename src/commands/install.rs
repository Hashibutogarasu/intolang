use anyhow::Result;
use clap::{ArgMatches, Args as ClapArgs, Command as ClapCommand, FromArgMatches};
use moduler::Project;
use std::path::PathBuf;

use crate::commands::command::Command;

#[derive(ClapArgs)]
struct InstallArgs {
    /// Package directory to install into the current project; without it, the
    /// packages recorded in into-package-lock.json are restored
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
        match args.source {
            Some(source) => {
                let package = project.install(&source)?;
                println!("Installed {} {}", package.alias, package.version);
            }
            None => {
                for restoration in project.restore()? {
                    let status = if restoration.installed {
                        "Installed"
                    } else {
                        "Up to date"
                    };
                    println!(
                        "{status} {} {}",
                        restoration.package.alias, restoration.package.version
                    );
                }
            }
        }
        Ok(())
    }
}
