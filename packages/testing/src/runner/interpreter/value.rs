use std::collections::HashMap;
use types::Storage;

/// A binding declared by a test: whether its memory exists depends on how it
/// was stored and whether an `init` block is running.
#[derive(Clone)]
pub struct Object {
    pub storage: Storage,
    pub allocated: bool,
    pub in_init: bool,
}

#[derive(Clone)]
pub enum Value {
    Bool(bool),
    Object(Object),
}

pub type Env = HashMap<String, Value>;

/// Mark every locally stored binding as inside or outside an `init` block.
pub fn set_init(env: &mut Env, in_init: bool) {
    for value in env.values_mut() {
        if let Value::Object(object) = value {
            if object.storage == Storage::Local {
                object.in_init = in_init;
            }
        }
    }
}
