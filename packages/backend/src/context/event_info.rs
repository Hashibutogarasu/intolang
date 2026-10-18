use ast::{Param, Stmt};

/// A declared event: the parameters its handlers take and the default handler
/// its body forms.
#[derive(Debug, Clone)]
pub struct EventInfo {
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
}
