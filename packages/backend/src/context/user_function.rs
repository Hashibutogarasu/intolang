use ast::{Param, Stmt};

/// A user defined function collected from the entry file or an imported module.
#[derive(Debug, Clone)]
pub struct UserFunction {
    pub name: String,
    pub receiver: Option<String>,
    pub is_static: bool,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
}
