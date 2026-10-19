use anyhow::Result;
use intovm::{ProcessInfo, ProcessState, TraceEntry, Vm, VmOptions};

use super::evaluate::eval_with;
use super::package::Package;

/// What an Into program did when the VM ran it.
#[derive(Debug, Clone)]
pub struct VmOutcome {
    pub stdout: String,
    pub trace: Vec<TraceEntry>,
    /// The virtual processes when the program ended, by pid.
    pub processes: Vec<ProcessInfo>,
    /// The pid of the virtual process the program was.
    pub current_pid: i32,
    /// The virtual clock when the program ended, in milliseconds.
    pub clock_ms: u64,
}

impl VmOutcome {
    pub fn process(&self, pid: i32) -> Option<&ProcessInfo> {
        self.processes.iter().find(|process| process.pid == pid)
    }

    /// How the program's own process ended.
    pub fn exit(&self) -> Option<ProcessState> {
        self.process(self.current_pid).map(|process| process.state)
    }
}

/// Lower Into source with the builtin `std` package and run it in the VM,
/// which needs no LLVM and starts no real process.
pub fn eval_vm(source: &str) -> Result<VmOutcome> {
    eval_vm_with(source, &[])
}

/// Like [`eval_vm`], with extra packages available for `import`.
pub fn eval_vm_with(source: &str, packages: &[Package]) -> Result<VmOutcome> {
    run_message(&eval_with(source, packages)?.message, VmOptions::default())
}

/// Like [`eval_vm`], with the packages the compiler bundles (`ps`, `json`,
/// ...) available for `import` and the VM set up by `options`.
#[cfg(feature = "run")]
pub fn eval_vm_bundled(source: &str, options: VmOptions) -> Result<VmOutcome> {
    let packages = compiler::builtin_packages_dir();
    let message = backend::lower_with_packages(source, Some(&packages))?;
    run_message(&message, options)
}

pub(crate) fn run_message(message: &str, options: VmOptions) -> Result<VmOutcome> {
    let mut vm = Vm::new(options);
    vm.load(message)?;
    vm.run()?;
    let processes = vm.processes();
    Ok(VmOutcome {
        stdout: vm.stdout(),
        trace: vm.trace().to_vec(),
        processes: processes.all().cloned().collect(),
        current_pid: processes.current_pid(),
        clock_ms: processes.clock(),
    })
}
