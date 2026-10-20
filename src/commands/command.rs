use clap::{ArgMatches, Command as ClapCommand, FromArgMatches};
use errors::{Errors, Result};

/// The typed arguments of a command, parsed from its matches.
pub(super) fn arguments<T: FromArgMatches>(matches: &ArgMatches) -> Result<T> {
    T::from_arg_matches(matches).map_err(|error| {
        Errors::Internal {
            message: error.to_string(),
        }
        .into()
    })
}

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
