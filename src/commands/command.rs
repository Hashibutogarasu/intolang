use anyhow::Result;
use clap::{ArgMatches, Command as ClapCommand};

/// A CLI command.
///
/// The trait deliberately holds no alias: each command's name and arguments are
/// declared exactly once, in the clap definition returned by [`Command::command`].
pub(super) trait Command {
    /// This command's clap definition (its name, arguments and flags).
    fn command(&self) -> ClapCommand;

    /// The subcommands this command owns, in display order.
    fn depends(&self) -> Vec<Box<dyn Command>>;

    /// Run this command using its parsed matches.
    fn execute(&self, matches: &ArgMatches) -> Result<()>;
}

/// Build a clap command tree from a root [`Command`] and everything it depends on.
pub(super) fn assemble(command: &dyn Command) -> ClapCommand {
    let mut clap = command.command();
    for child in command.depends() {
        clap = clap.subcommand(assemble(child.as_ref()));
    }
    clap
}
