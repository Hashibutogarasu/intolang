use ast::{Param, Stmt};

/// One of several functions that share a name: the key it is stored under,
/// the written types of its parameters and how many arguments a call has to
/// give.
#[derive(Debug, Clone)]
pub struct Overload {
    pub key: String,
    pub params: Vec<Option<String>>,
    pub required: usize,
}

/// A user defined function collected from the entry file or an imported module.
#[derive(Debug, Clone)]
pub struct UserFunction {
    /// The key the function is stored and called under; `base` with the types
    /// of its parameters when other functions share the name.
    pub name: String,
    /// The name or `Class.method` without the types of the parameters.
    pub base: String,
    pub receiver: Option<String>,
    pub is_static: bool,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub body: Vec<Stmt>,
    /// Whether the function only reads the tracking state of its arguments,
    /// so that it may be given a moved or destroyed binding.
    pub inspects: bool,
}

impl UserFunction {
    /// Whether the function returns a reference (`T&`) that owns nothing.
    pub fn returns_reference(&self) -> bool {
        self.return_type
            .as_deref()
            .is_some_and(|ty| ty.ends_with('&'))
    }
}
