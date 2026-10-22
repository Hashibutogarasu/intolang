use clap::{ArgMatches, Args as ClapArgs, Command as ClapCommand};
use errors::Result;
use moduler::{PackageInfo, Project};
use std::path::Path;

use crate::commands::command::{Command, arguments};
use crate::commands::package_task::{Operation, Outcome, PackageStatus, PackageTask};

#[derive(ClapArgs)]
struct UpdateArgs {
    /// Aliases of the packages to update; without them every locked package is
    /// checked
    packages: Vec<String>,
}

pub(super) struct Update;

struct UpdateTask {
    packages: Vec<String>,
}

impl PackageTask for UpdateTask {
    type Item = PackageInfo;

    fn operation(&self) -> Operation {
        Operation::Update
    }

    fn detect(&self, project: &Project, _embedded: &Path) -> Result<Vec<PackageInfo>> {
        Ok(project
            .locked()?
            .into_iter()
            .filter(|package| self.packages.is_empty() || self.packages.contains(&package.alias))
            .collect())
    }

    fn label(&self, package: &PackageInfo) -> Outcome {
        Outcome::new(PackageStatus::Updating, &package.alias)
    }

    fn handle(&self, project: &Project, embedded: &Path, package: PackageInfo) -> Result<Outcome> {
        let update = project.update(&package, embedded)?;
        let status = if update.updated {
            PackageStatus::Updated
        } else {
            PackageStatus::UpToDate
        };
        Ok(Outcome::new(
            status,
            format!("{} {}", update.package.alias, update.package.version),
        ))
    }
}

impl Command for Update {
    fn command(&self) -> ClapCommand {
        UpdateArgs::augment_args(
            ClapCommand::new("update")
                .about("Overwrite the installed packages whose source has changed"),
        )
    }

    fn depends(&self) -> Vec<Box<dyn Command>> {
        Vec::new()
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let args = arguments::<UpdateArgs>(matches)?;
        let project = Project::new(std::env::current_dir()?);
        let embedded = compiler::builtin_packages_dir();
        UpdateTask {
            packages: args.packages,
        }
        .run(&project, &embedded)
    }
}
