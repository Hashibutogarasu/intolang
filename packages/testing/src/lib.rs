mod eval;
mod runner;

pub use eval::{
    Evaluation, Execution, Package, VmOutcome, Workspace, eval, eval_vm, eval_vm_with, eval_with,
};
#[cfg(feature = "run")]
pub use eval::{eval_run, eval_vm_bundled};
pub use runner::{TestSummary, run_tests};
