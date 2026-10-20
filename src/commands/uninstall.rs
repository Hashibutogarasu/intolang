use clap::{ArgMatches, Args as ClapArgs, Command as ClapCommand};
use errors::Result;
use moduler::Project;
use progress::Progress;

use crate::commands::command::{Command, arguments};

#[derive(ClapArgs)]
struct UninstallArgs {
    /// Aliases of the packages to remove
    #[arg(required = true)]
    packages: Vec<String>,
}

pub(super) struct Uninstall;

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
        let progress = Progress::start("Looking for the project", args.packages.len() as u64);
        let project = Project::new(std::env::current_dir()?);
        let mut removed = Vec::new();
        for alias in &args.packages {
            progress.step(&format!("Removing {alias}"));
            let package = project.uninstall(alias)?;
            removed.push(format!("{} {}", package.alias, package.version));
        }
        progress.finish(&format!("Removed {}", removed.join(", ")));
        Ok(())
    }
}
