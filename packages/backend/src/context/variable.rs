use types::Type;

/// What has happened to the instance a binding holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ownership {
    #[default]
    Live,
    Moved,
    Destroyed,
    Detached,
}

/// How a binding relates to the instance it holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tracking {
    pub state: Ownership,
    /// The binding this one borrows from, for `x.borrow()`.
    pub borrows: Option<String>,
    /// How many live borrows were taken from this binding.
    pub borrowed_by: usize,
    /// Whether the binding is a reference (`T&`) that owns nothing.
    pub reference: bool,
    /// Whether the state changed inside a nested scope, so that whether the
    /// instance is still owned is only known at run time (its drop flag).
    pub conditional: bool,
}

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
    pub constant: bool,
    pub tracking: Tracking,
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
            constant: false,
            tracking: Tracking::default(),
        }
    }
}
