mod eval;
mod runner;

#[cfg(feature = "run")]
pub use eval::eval_run;
pub use eval::{Evaluation, Execution, Package, Workspace, eval, eval_with};
pub use runner::{TestSummary, run_tests};
