use std::path::Path;

use intovm::{Vm, VmOptions};
use ipc::Severity;
use serde::Serialize;

use crate::Session;
use crate::builtin::{PACKAGES_DIR, builtin_files};

/// What a program printed, and why it stopped when it did not finish.
#[derive(Debug, Clone, Serialize)]
pub struct RunOutcome {
    pub stdout: String,
    pub error: Option<String>,
}

/// Lower `source` against the standard package and run it in the VM, which
/// starts no process and sleeps on a clock of its own. A program the editor
/// underlines in red does not run.
pub fn run(source: &str, stdin: &str) -> RunOutcome {
    if let Some(error) = Session::new()
        .diagnose(source)
        .into_iter()
        .find(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return RunOutcome {
            stdout: String::new(),
            error: Some(format!(
                "line {}: {}",
                error.range.start.line + 1,
                error.message
            )),
        };
    }
    package::with_vfs(builtin_files(), || {
        let mut vm = Vm::new(VmOptions {
            stdin: Some(stdin.as_bytes().to_vec()),
            ..VmOptions::default()
        });
        let result = backend::lower_with_packages(source, Some(Path::new(PACKAGES_DIR)))
            .and_then(|message| vm.load(&message))
            .and_then(|()| vm.run());
        RunOutcome {
            stdout: vm.stdout(),
            error: result.err().map(|error| error.to_string()),
        }
    })
}
