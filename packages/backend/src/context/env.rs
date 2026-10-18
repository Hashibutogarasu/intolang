use moduler::Module;
use native::NativeLink;
use std::collections::{HashMap, HashSet};

use super::class_def::{ClassFields, EnumValues};
use super::class_info::Classes;
use super::user_function::UserFunction;

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
    pub type_params: HashSet<String>,
    pub imported_functions: HashSet<String>,
}

impl Env {
    pub fn new(source_path: impl Into<String>) -> Self {
        Self {
            source_path: source_path.into(),
            ..Self::default()
        }
    }
}
