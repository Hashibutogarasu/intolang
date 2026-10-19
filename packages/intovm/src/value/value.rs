/// A value a slot, field or argument holds in the VM.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(Vec<u8>),
    Ref(usize),
    Native { ptr: usize, class: String },
    Unit,
}

impl Value {
    pub fn int(&self) -> Option<i64> {
        match self {
            Value::Int(number) => Some(*number),
            _ => None,
        }
    }

    pub fn float(&self) -> Option<f64> {
        match self {
            Value::Float(number) => Some(*number),
            Value::Int(number) => Some(*number as f64),
            _ => None,
        }
    }
}
