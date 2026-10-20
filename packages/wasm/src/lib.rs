//! The compiler and the language server for a browser: the standard package
//! travels inside the module, and no file or process of the computer is used.

mod builtin;
mod run;
mod session;
mod web;

pub use builtin::{PACKAGES_DIR, builtin_files, read_builtin};
pub use run::{RunOutcome, run};
pub use session::Session;
