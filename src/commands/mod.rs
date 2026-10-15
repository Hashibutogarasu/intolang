mod build;
mod format;
mod new;

use anyhow::Result;
use std::path::PathBuf;

pub(super) fn run() -> Result<()> {
    let mut argv = std::env::args_os();
    let _program = argv.next();
    let command = argv.next();
    if command.as_deref().is_some_and(|arg| arg == "new") {
        return new::run();
    }
    let explicit_build = command.as_deref().is_some_and(|arg| arg == "build");
    let args = Args::parse(explicit_build);
    if args.format_mode {
        format::run(args.package, args.format_depth)
    } else {
        build::run(args.package, args.test_mode, args.run_mode)
    }
}

#[derive(clap::Parser)]
#[command(name = "intolang", about = "Build an Into package")]
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
    package: PathBuf,
}

impl Args {
    fn parse(explicit_build: bool) -> Self {
        use clap::Parser;
        use std::ffi::OsString;

        let argv = std::env::args_os().enumerate().filter_map(|(index, arg)| {
            if explicit_build && index == 1 { return None; }
            Some(match arg.to_str() {
            Some("-test") => OsString::from("--test"),
            Some("-format") => OsString::from("--format"),
            Some("-depth") => OsString::from("--depth"),
            Some("-run") => OsString::from("--run"),
            Some(value) if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 => {
                OsString::from(&value[1..value.len() - 1])
            }
            _ => arg,
            })
        });
        Self::parse_from(argv)
    }
}
