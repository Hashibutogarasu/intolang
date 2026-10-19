use std::collections::HashMap;

use crate::value::Value;

/// One running function. The main frame keeps its slots in the globals; a
/// handler frame has its own and reaches the slots it captures in the globals.
#[derive(Debug, Clone)]
pub struct Frame {
    pub pc: usize,
    pub locals: HashMap<String, Value>,
    pub captures: Vec<String>,
    pub main: bool,
}

impl Frame {
    pub fn main() -> Self {
        Self {
            pc: 0,
            locals: HashMap::new(),
            captures: Vec::new(),
            main: true,
        }
    }

    pub fn shares(&self, name: &str) -> bool {
        self.main || self.captures.iter().any(|capture| capture == name)
    }
}
