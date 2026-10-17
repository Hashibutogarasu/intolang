use std::collections::{HashMap, HashSet};

use crate::imports::{self, NativeLink};
use crate::syntax::support::text::diagnostic;
use crate::types::Type;

/// A lowered local binding tracked while parsing statements.
#[derive(Debug, Clone)]
pub struct Variable {
    pub value: String,
    pub ty: Type,
    pub memory_backed: bool,
    pub allocated: bool,
    pub lazy: bool,
    pub unloaded: bool,
    pub declared_in_init: bool,
    pub native: bool,
}

/// A formally declared class field: `pub into: <Type> <name> = <default>;`.
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub ty: String,
    pub default: Option<String>,
    pub is_static: bool,
}

pub type ClassFields = HashMap<String, Vec<FieldDef>>;

/// A user defined function collected from the entry file or an imported module.
#[derive(Clone)]
pub struct UserFunction {
    pub name: String,
    pub receiver: Option<String>,
    pub is_static: bool,
    pub params: Vec<imports::Param>,
    pub body: Vec<String>,
}

/// Shared mutable state threaded through every syntax construct while parsing.
pub struct ParseContext {
    pub source_path: String,
    pub values: HashMap<String, Variable>,
    pub native_links: HashMap<String, NativeLink>,
    pub enum_values: HashMap<String, HashMap<String, i32>>,
    pub class_fields: ClassFields,
    pub user_functions: HashMap<String, UserFunction>,
    pub type_params: HashSet<String>,
    pub imported_functions: HashSet<String>,
    pub output: String,
    pub temp_counter: usize,
    pub in_init: bool,
    pub init_has_run: bool,
    pub blocks: Vec<char>,
}

impl ParseContext {
    pub fn new(source_path: impl Into<String>) -> Self {
        Self {
            source_path: source_path.into(),
            values: HashMap::new(),
            native_links: HashMap::new(),
            enum_values: HashMap::new(),
            class_fields: HashMap::new(),
            user_functions: HashMap::new(),
            type_params: HashSet::new(),
            imported_functions: HashSet::new(),
            output: String::new(),
            temp_counter: 0,
            in_init: false,
            init_has_run: false,
            blocks: Vec::new(),
        }
    }

    /// The human readable scope used in diagnostics.
    pub fn scope_name(&self) -> &'static str {
        if self.in_init { "init" } else { "module" }
    }

    /// Build a diagnostic bound to this context's source path.
    pub fn diagnostic(&self, line: usize, message: &str) -> anyhow::Error {
        diagnostic(&self.source_path, line, self.scope_name(), message)
    }
}
