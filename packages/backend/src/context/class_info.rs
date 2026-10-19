use std::collections::HashMap;

/// The name every class without an explicit parent inherits from.
pub const OBJECT: &str = "Object";

/// The built-in class of a declared event used as a value.
pub const EVENT: &str = "Event";

/// The built-in class of a registration made with `on`.
pub const LISTENER: &str = "Listener";

/// The built-in class of a `thread { .. }` expression.
pub const THREAD: &str = "Thread";

/// The built-in class of a started thread.
pub const TASK: &str = "Task";

/// A method or event member of a class, reduced to what inheritance checks
/// and resolution need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberInfo {
    pub name: String,
    pub line: usize,
    pub is_static: bool,
    pub is_abstract: bool,
    pub is_default: bool,
    pub is_override: bool,
    /// The type arguments written after the name (`onCast<Char, Hex>`).
    pub type_params: Vec<String>,
    /// The written parameter types; an unwritten type is the empty string.
    pub params: Vec<String>,
    pub return_type: Option<String>,
    pub has_body: bool,
    pub is_native: bool,
}

impl MemberInfo {
    /// Whether a subclass may redeclare the member with `override`.
    pub fn is_open(&self) -> bool {
        self.is_abstract || self.is_default || self.is_override
    }
}

/// A declared class as inheritance sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassInfo {
    pub name: String,
    pub line: usize,
    pub is_abstract: bool,
    /// The type parameters of a generic class, `T` for `Box<T>()`.
    pub type_params: Vec<String>,
    pub parent: Option<String>,
    /// The parent as written, with its type arguments.
    pub parent_type: Option<String>,
    pub methods: Vec<MemberInfo>,
    pub events: Vec<MemberInfo>,
}

pub type Classes = HashMap<String, ClassInfo>;
