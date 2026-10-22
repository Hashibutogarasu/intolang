use clap::{ArgMatches, Args, Command as ClapCommand};
use errors::{ReportFormat, Result};
use std::path::PathBuf;
use std::process::ExitCode;

use crate::commands::build::{self, Build, Run};
use crate::commands::command::{Command, arguments, assemble};
use crate::commands::error_format::ErrorFormat;
use crate::commands::format::{self, Format};
use crate::commands::install::Install;
use crate::commands::new::New;
use crate::commands::uninstall::Uninstall;
use crate::commands::update::Update;

// The global arguments, used when no subcommand is given.
#[derive(Args)]
struct RootArgs {
    #[arg(long = "test")]
    test: bool,
    #[arg(long = "run")]
    run: bool,
    /// Write an .inex file for the Into VM instead of a native executable
    #[arg(long = "vm")]
    vm: bool,
    #[arg(long = "format")]
    format: bool,
    /// How failures are written: for people on stderr, or as JSON on stdout
    #[arg(
        long = "error-format",
        global = true,
        value_enum,
        default_value = "human"
    )]
    error_format: ErrorFormat,
    /// Maximum directory depth to search in format mode (0 means this directory only)
    #[arg(long = "depth", requires = "format")]
    depth: Option<usize>,
    /// Package directory, path to its into-package.json or an .inex file to run
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
            Box::new(Update),
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
        let args = arguments::<RootArgs>(matches)?;
        if args.format {
            return format::run(args.package, args.depth);
        }
        build::run(args.package, args.test, args.run, args.vm)
    }
}

pub(crate) fn run() -> ExitCode {
    let argv = normalize();
    let root = Root;
    let mut command = assemble(&root);
    if argv.len() <= 1 {
        return match command.print_help() {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
    }
    let matches = command.get_matches_from(argv);
    let format = matches
        .get_one::<ErrorFormat>("error_format")
        .copied()
        .unwrap_or_default();
    match root.execute(&matches) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            ReportFormat::from(format)
                .reporter()
                .emit_with(&error, &progress::fail);
            ExitCode::FAILURE
        }
    }
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
