mod build;
mod format;
mod install;
mod new;

use anyhow::Result;
use std::path::PathBuf;

pub(super) fn run() -> Result<()> {
    let mut argv = std::env::args_os();
    let _program = argv.next();
    let command = argv.next();
    if command.is_none() || command.as_deref().is_some_and(|arg| arg == "help") {
        use clap::CommandFactory;
        return Args::command().print_help().map_err(Into::into);
    }
    if command.as_deref().is_some_and(|arg| arg == "new") {
        return new::run();
    }
    if command.as_deref().is_some_and(|arg| arg == "install") {
        return install::run();
    }
    let command_name = command.as_deref().and_then(|arg| arg.to_str());
    let explicit_command = command_name.filter(|name| matches!(*name, "build" | "run"));
    let args = Args::parse(explicit_command);
    if args.format_mode {
        format::run(args.package, args.format_depth)
    } else {
        build::run(args.package, args.test_mode, args.run_mode)
    }
}

#[derive(clap::Parser)]
#[command(name = "intolang", about = "Build or run an Into package")]
struct Args {
    #[arg(long = "test")]
    test_mode: bool,
    #[arg(long = "run")]
    run_mode: bool,
    #[arg(long = "format")]
    format_mode: bool,
    /// Maximum directory depth to search in format mode (0 means this directory only)
    #[arg(long = "depth", requires = "format_mode")]
    format_depth: Option<usize>,
    /// Package directory or path to its into-package.json
    #[arg(default_value = ".")]
    package: PathBuf,
}

impl Args {
    fn parse(explicit_command: Option<&str>) -> Self {
        use clap::Parser;
        use std::ffi::OsString;

        let mut raw_args = std::env::args_os();
        let mut argv = vec![raw_args.next().unwrap_or_default()];
        if explicit_command.is_some() {
            raw_args.next();
        }
        if explicit_command == Some("run") {
            argv.push(OsString::from("--run"));
        }
        argv.extend(raw_args.map(|arg| match arg.to_str() {
            Some("-test") => OsString::from("--test"),
            Some("-format") => OsString::from("--format"),
            Some("-depth") => OsString::from("--depth"),
            Some("-run") => OsString::from("--run"),
            Some(value) if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 => {
                OsString::from(&value[1..value.len() - 1])
            }
            _ => arg,
        }));
        Self::parse_from(argv)
    }
}
