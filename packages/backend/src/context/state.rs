use std::collections::HashMap;

use super::variable::{Tracking, Variable};

/// What lowering changes as it walks statements.
#[derive(Default)]
pub struct State {
    pub values: HashMap<String, Variable>,
    /// The tracking the next declared binding starts with, set by `copy`,
    /// `move` and `borrow` while their value is lowered.
    pub pending_tracking: Option<Tracking>,
    pub output: String,
    pub temp_counter: usize,
    pub in_init: bool,
    pub init_has_run: bool,
    pub loop_depth: usize,
    /// For each open scope, the bindings it declared in order.
    pub scopes: Vec<Vec<String>>,
    /// The index in `scopes` of the body scope of each open loop.
    pub loop_scopes: Vec<usize>,
}

impl State {
    /// The human readable scope used in diagnostics.
    pub fn scope_name(&self) -> &'static str {
        if self.in_init { "init" } else { "module" }
    }
}
