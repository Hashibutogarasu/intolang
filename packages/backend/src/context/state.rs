use errors::Error;
use std::collections::{HashMap, HashSet};
use types::Type;

use super::event_info::EventInfo;
use super::variable::{Tracking, Variable};

/// An inlined function call being lowered.
#[derive(Debug, Clone, Copy)]
pub struct InlineFrame {
    /// The index in `State::scopes` of the scope the call opened.
    pub scope_base: usize,
    pub returns_reference: bool,
}

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
    /// The inlined function calls being lowered, innermost last.
    pub frames: Vec<InlineFrame>,
    pub destroyed_events: HashSet<String>,
    /// The events of class members reached so far, by the path to them such as
    /// `http.on.response`; each binding has its own.
    pub member_events: HashMap<String, EventInfo>,
    /// The functions called at run time whose definition is in the output.
    pub defined_functions: HashSet<String>,
    /// The types the run time functions being lowered return, innermost last.
    pub returning: Vec<Type>,
    pub collected: Option<Vec<Error>>,
    pub poisoned: HashSet<String>,
}

impl State {
    /// The human readable scope used in diagnostics.
    pub fn scope_name(&self) -> &'static str {
        if self.in_init { "init" } else { "module" }
    }
}
