use clap::{ArgMatches, Args as ClapArgs, Command as ClapCommand};
use errors::Result;
use moduler::Project;
use std::path::Path;

use crate::commands::command::{Command, arguments};
use crate::commands::package_task::{Operation, Outcome, PackageStatus, PackageTask};

#[derive(ClapArgs)]
struct UninstallArgs {
    /// Aliases of the packages to remove
    #[arg(required = true)]
    packages: Vec<String>,
}

pub(super) struct Uninstall;

struct UninstallTask {
    packages: Vec<String>,
}

impl PackageTask for UninstallTask {
    type Item = String;

    fn operation(&self) -> Operation {
        Operation::Uninstall
    }

    fn detect(&self, _project: &Project, _embedded: &Path) -> Result<Vec<String>> {
        Ok(self.packages.clone())
    }

    fn label(&self, alias: &String) -> Outcome {
        Outcome::new(PackageStatus::Removing, alias)
    }

    fn handle(&self, project: &Project, _embedded: &Path, alias: String) -> Result<Outcome> {
        let package = project.uninstall(&alias)?;
        Ok(Outcome::new(
            PackageStatus::Removed,
            format!("{} {}", package.alias, package.version),
        ))
    }
}

impl Command for Uninstall {
    fn command(&self) -> ClapCommand {
        UninstallArgs::augment_args(
            ClapCommand::new("uninstall")
                .visible_alias("rm")
                .about("Remove packages from the current project"),
        )
    }

    fn depends(&self) -> Vec<Box<dyn Command>> {
        Vec::new()
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let args = arguments::<UninstallArgs>(matches)?;
        let project = Project::new(std::env::current_dir()?);
        UninstallTask {
            packages: args.packages,
        }
        .run(&project, Path::new(""))
    }
}
