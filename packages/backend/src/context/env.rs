use moduler::Module;
use native::NativeLink;
use std::collections::{HashMap, HashSet};

use super::class_def::{ClassFields, EnumValues};
use super::class_info::Classes;
use super::event_info::EventInfo;
use super::user_function::{Overload, UserFunction};
use super::variable::Ownership;

/// Everything lowering reads but never changes: the declarations collected
/// from the entry file and the modules it imports.
#[derive(Default)]
pub struct Env {
    pub source_path: String,
    pub native_links: HashMap<String, NativeLink>,
    pub modules: Vec<Module>,
    pub enum_values: EnumValues,
    pub class_fields: ClassFields,
    pub classes: Classes,
    pub user_functions: HashMap<String, UserFunction>,
    /// The functions that share each name, by the name or `Class.method`.
    pub overloads: HashMap<String, Vec<Overload>>,
    /// The keys of the functions that are called at run time.
    pub runtime_functions: HashSet<String>,
    pub consuming: HashMap<String, Ownership>,
    pub events: HashMap<String, EventInfo>,
    pub type_params: HashSet<String>,
    pub imported_functions: HashSet<String>,
    /// Whether a call to a function that was never imported is an error
    /// instead of producing no output.
    pub strict_calls: bool,
}

impl Env {
    pub fn new(source_path: impl Into<String>) -> Self {
        Self {
            source_path: source_path.into(),
            ..Self::default()
        }
    }
}
