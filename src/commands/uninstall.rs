use anyhow::Result;
use clap::{ArgMatches, Args as ClapArgs, Command as ClapCommand, FromArgMatches};
use moduler::Project;

use crate::commands::command::Command;

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
        let args = UninstallArgs::from_arg_matches(matches)?;
        let project = Project::new(std::env::current_dir()?);
        for alias in &args.packages {
            let package = project.uninstall(alias)?;
            println!("Removed {} {}", package.alias, package.version);
        }
        Ok(())
    }
}
