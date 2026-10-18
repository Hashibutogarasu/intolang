use types::Type;

/// A lowered local binding tracked while lowering statements.
#[derive(Debug, Clone)]
pub struct Variable {
    pub value: String,
    pub ty: Type,
    pub memory_backed: bool,
    pub allocated: bool,
    pub lazy: bool,
    pub unloaded: bool,
    pub declared_in_init: bool,
    pub native: bool,
}

impl Variable {
    /// A compiler generated temporary holding a native value of type `ty`.
    pub fn temporary(ty: Type) -> Self {
        Self {
            value: ty.name(),
            ty,
            memory_backed: false,
            allocated: true,
            lazy: false,
            unloaded: false,
            declared_in_init: false,
            native: true,
        }
    }
}
