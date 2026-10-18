use std::collections::HashMap;
use types::Storage;

/// What has happened to the instance a binding holds.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum State {
    Live,
    Moved,
    Destroyed,
}

/// A binding declared by a test: whether its memory exists depends on how it
/// was stored and whether an `init` block is running, and who owns its
/// instance depends on what was copied, moved, borrowed or destroyed.
#[derive(Clone)]
pub struct Object {
    pub storage: Storage,
    pub allocated: bool,
    pub in_init: bool,
    pub number: Option<i64>,
    pub state: State,
    pub reference: bool,
    pub borrowed_by: usize,
    pub borrows: Option<String>,
}

impl Object {
    pub fn declared(storage: Storage, in_init: bool, number: Option<i64>) -> Self {
        Self {
            storage,
            allocated: storage == Storage::Now,
            in_init,
            number,
            state: State::Live,
            reference: false,
            borrowed_by: 0,
            borrows: None,
        }
    }
}

#[derive(Clone)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Object(Object),
}

impl Value {
    /// The number a value stands for, if it is one.
    pub fn number(&self) -> Option<i64> {
        match self {
            Value::Int(number) => Some(*number),
            Value::Object(object) => object.number,
            Value::Bool(_) => None,
        }
    }
}

pub type Env = HashMap<String, Value>;

/// Mark every locally stored binding as inside or outside an `init` block.
pub fn set_init(env: &mut Env, in_init: bool) {
    for value in env.values_mut() {
        if let Value::Object(object) = value
            && object.storage == Storage::Local
        {
            object.in_init = in_init;
        }
    }
}
