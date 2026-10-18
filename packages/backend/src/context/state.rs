use std::collections::HashMap;

use super::variable::Variable;

/// What lowering changes as it walks statements.
#[derive(Default)]
pub struct State {
    pub values: HashMap<String, Variable>,
    pub output: String,
    pub temp_counter: usize,
    pub in_init: bool,
    pub init_has_run: bool,
    pub loop_depth: usize,
}

impl State {
    /// The human readable scope used in diagnostics.
    pub fn scope_name(&self) -> &'static str {
        if self.in_init { "init" } else { "module" }
    }
}
