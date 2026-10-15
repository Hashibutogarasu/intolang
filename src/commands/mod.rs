mod build;
mod format;
mod new;

use anyhow::Result;
use std::path::PathBuf;

pub(super) fn run() -> Result<()> {
    let mut argv = std::env::args_os();
    let _program = argv.next();
    if argv.next().as_deref().is_some_and(|arg| arg == "new") {
        return new::run();
    }
    let args = Args::parse();
    if args.format_mode {
        format::run(args.package, args.format_depth)
    } else {
        build::run(args.package, args.test_mode)
    }
}

#[derive(clap::Parser)]
#[command(name = "intolang", about = "Build an Into package")]
struct Args {
    #[arg(long = "test")]
    test_mode: bool,
    #[arg(long = "format")]
    format_mode: bool,
    /// Maximum directory depth to search in format mode (0 means this directory only)
    #[arg(long = "depth", requires = "format_mode")]
    format_depth: Option<usize>,
    /// Package directory or path to its into-package.json
    package: PathBuf,
}

impl Args {
    fn parse() -> Self {
        use clap::Parser;
        use std::ffi::OsString;

        let argv = std::env::args_os().map(|arg| match arg.to_str() {
            Some("-test") => OsString::from("--test"),
            Some("-format") => OsString::from("--format"),
            Some("-depth") => OsString::from("--depth"),
            _ => arg,
        });
        Self::parse_from(argv)
    }
}
