use anyhow::Result;
use clap::{ArgMatches, Args, Command as ClapCommand, FromArgMatches};
use std::path::PathBuf;

use crate::commands::build::{self, Build, Run};
use crate::commands::command::{Command, assemble};
use crate::commands::format::{self, Format};
use crate::commands::install::Install;
use crate::commands::new::New;
use crate::commands::uninstall::Uninstall;

// The global arguments, used when no subcommand is given.
#[derive(Args)]
struct RootArgs {
    #[arg(long = "test")]
    test: bool,
    #[arg(long = "run")]
    run: bool,
    #[arg(long = "format")]
    format: bool,
    /// Maximum directory depth to search in format mode (0 means this directory only)
    #[arg(long = "depth", requires = "format")]
    depth: Option<usize>,
    /// Package directory or path to its into-package.json
    #[arg(default_value = ".")]
    package: PathBuf,
}

/// The top-level command. It owns the global arguments and the subcommands, and
/// has no alias of its own.
pub(super) struct Root;

impl Command for Root {
    fn command(&self) -> ClapCommand {
        RootArgs::augment_args(
            ClapCommand::new("intolang")
                .about("Build or run an Into package")
                .subcommand_precedence_over_arg(true),
        )
    }

    fn depends(&self) -> Vec<Box<dyn Command>> {
        vec![
            Box::new(Build),
            Box::new(Run),
            Box::new(Format),
            Box::new(New),
            Box::new(Install),
            Box::new(Uninstall),
        ]
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        if let Some((name, sub_matches)) = matches.subcommand() {
            for child in self.depends() {
                if child.command().get_name() == name {
                    return child.execute(sub_matches);
                }
            }
        }
        let args = RootArgs::from_arg_matches(matches)?;
        if args.format {
            return format::run(args.package, args.depth);
        }
        build::run(args.package, args.test, args.run)
    }
}

pub(crate) fn run() -> Result<()> {
    let argv = normalize();
    let root = Root;
    let mut command = assemble(&root);
    if argv.len() <= 1 {
        return command.print_help().map_err(Into::into);
    }
    let matches = command.get_matches_from(argv);
    root.execute(&matches)
}

fn normalize() -> Vec<std::ffi::OsString> {
    let raw: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let mut argv = Vec::with_capacity(raw.len());
    for (index, arg) in raw.into_iter().enumerate() {
        if index == 0 {
            argv.push(arg);
            continue;
        }
        argv.push(match arg.to_str() {
            Some("-test") => "--test".into(),
            Some("-format") => "--format".into(),
            Some("-depth") => "--depth".into(),
            Some("-run") => "--run".into(),
            Some(value) if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 => {
                value[1..value.len() - 1].to_owned().into()
            }
            _ => arg,
        });
    }
    argv
}
