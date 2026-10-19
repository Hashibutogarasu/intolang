use ast::{Param, Stmt};

/// A user defined function collected from the entry file or an imported module.
#[derive(Debug, Clone)]
pub struct UserFunction {
    pub name: String,
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
