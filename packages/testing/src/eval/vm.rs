use anyhow::Result;
use intovm::{TraceEntry, Vm, VmOptions};

use super::evaluate::eval_with;
use super::package::Package;

/// What an Into program did when the VM ran it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmOutcome {
    pub stdout: String,
    pub trace: Vec<TraceEntry>,
}

/// Lower Into source with the builtin `std` package and run it in the VM,
/// which needs no LLVM and starts no real process.
pub fn eval_vm(source: &str) -> Result<VmOutcome> {
    eval_vm_with(source, &[])
}

/// Like [`eval_vm`], with extra packages available for `import`.
pub fn eval_vm_with(source: &str, packages: &[Package]) -> Result<VmOutcome> {
    run_message(&eval_with(source, packages)?.message)
}

pub(crate) fn run_message(message: &str) -> Result<VmOutcome> {
    let mut vm = Vm::new(VmOptions::default());
    vm.load(message)?;
    vm.run()?;
    Ok(VmOutcome {
        stdout: vm.stdout(),
        trace: vm.trace().to_vec(),
    })
}
