use clap::{ArgMatches, Args as ClapArgs, Command as ClapCommand};
use errors::{Errors, Result};
use moduler::{Project, find_by_alias, is_builtin};
use progress::Progress;
use std::path::PathBuf;

use crate::commands::command::{Command, arguments};

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
        let args = arguments::<InstallArgs>(matches)?;
        let progress = Progress::start("Looking for the project", 1);
        let project = Project::new(std::env::current_dir()?);
        let embedded = compiler::builtin_packages_dir();
        match args.source {
            Some(source) => {
                progress.step(&format!("Installing {}", source.display()));
                let source = match source.to_str() {
                    Some(name) if is_builtin(name) && !source.exists() => {
                        find_by_alias(&embedded, moduler::BUILTIN_ALIAS)?.ok_or_else(|| {
                            Errors::PackageNotEmbedded {
                                alias: moduler::BUILTIN_ALIAS.to_string(),
                            }
                        })?
                    }
                    _ => source,
                };
                let package = project.install(&source, &embedded)?;
                progress.finish(&format!("Installed {} {}", package.alias, package.version));
            }
            None => {
                progress.step("Restoring the packages");
                let restored = project
                    .restore(&embedded)?
                    .into_iter()
                    .map(|restoration| {
                        let status = if restoration.installed {
                            "Installed"
                        } else {
                            "Up to date"
                        };
                        format!(
                            "{status} {} {}\n",
                            restoration.package.alias, restoration.package.version
                        )
                    })
                    .collect::<Vec<_>>();
                progress.finish(&restored.join(""));
            }
        }
        Ok(())
    }
}
