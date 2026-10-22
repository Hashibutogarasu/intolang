use clap::{ArgMatches, Args as ClapArgs, Command as ClapCommand};
use errors::{Errors, Result};
use moduler::{Project, Restoration, is_builtin};
use package::Package;
use std::path::{Path, PathBuf};

use crate::commands::command::{Command, arguments};
use crate::commands::package_task::{Operation, Outcome, PackageStatus, PackageTask};

#[derive(ClapArgs)]
struct InstallArgs {
    /// Package directory to install into the current project; without it, the
    /// packages recorded in into-package-lock.json are restored. `builtin` (or
    /// `intolang`) references the package embedded in the compiler
    source: Option<PathBuf>,
}

pub(super) struct Install;

/// A package to install: the one named on the command line, or one restored
/// from the lock file.
enum InstallItem {
    Source(PathBuf),
    Restored(Restoration),
}

struct InstallTask {
    source: Option<PathBuf>,
}

impl PackageTask for InstallTask {
    type Item = InstallItem;

    fn operation(&self) -> Operation {
        Operation::Install
    }

    fn detect(&self, project: &Project, embedded: &Path) -> Result<Vec<InstallItem>> {
        match &self.source {
            Some(source) => Ok(vec![InstallItem::Source(source.clone())]),
            None => Ok(project
                .restore(embedded)?
                .into_iter()
                .map(InstallItem::Restored)
                .collect()),
        }
    }

    fn label(&self, item: &InstallItem) -> Outcome {
        match item {
            InstallItem::Source(source) => {
                Outcome::new(PackageStatus::Installing, source.display().to_string())
            }
            InstallItem::Restored(restoration) => {
                Outcome::new(PackageStatus::Restoring, &restoration.package.alias)
            }
        }
    }

    fn handle(&self, project: &Project, embedded: &Path, item: InstallItem) -> Result<Outcome> {
        match item {
            InstallItem::Source(source) => {
                let source = match source.to_str() {
                    Some(name) if is_builtin(name) && !source.exists() => {
                        Package::find_by_alias(embedded, moduler::BUILTIN_ALIAS)?
                            .ok_or_else(|| Errors::PackageNotEmbedded {
                                alias: moduler::BUILTIN_ALIAS.to_string(),
                            })?
                            .root
                    }
                    _ => source,
                };
                let package = project.install(&source, embedded)?;
                Ok(Outcome::new(
                    PackageStatus::Installed,
                    format!("{} {}", package.alias, package.version),
                ))
            }
            InstallItem::Restored(restoration) => {
                let status = if restoration.installed {
                    PackageStatus::Installed
                } else {
                    PackageStatus::UpToDate
                };
                Ok(Outcome::new(
                    status,
                    format!(
                        "{} {}",
                        restoration.package.alias, restoration.package.version
                    ),
                ))
            }
        }
    }
}

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
        let project = Project::new(std::env::current_dir()?);
        let embedded = compiler::builtin_packages_dir();
        InstallTask {
            source: args.source,
        }
        .run(&project, &embedded)
    }
}
