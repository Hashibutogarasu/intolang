mod eval;
mod runner;

pub use eval::{Evaluation, Package, Workspace, eval, eval_with};
pub use runner::{TestSummary, run_tests};
