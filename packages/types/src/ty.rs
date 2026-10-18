use std::collections::HashMap;
use std::fmt;

/// An explicit Into type as written in source, mirroring Rust's primitive set.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Void,
    Boolean,
    String,
    Path,
    Int8,
    Int16,
    Int32,
    Int64,
    Float32,
    Float64,
    Pointer,
    /// A pointer to another type, written `T*`.
    PointerTo(Box<Type>),
    /// A growable list of another type, written `List<T>`.
    List(Box<Type>),
    /// A generic type parameter such as `T`.
    Generic(String),
    /// A user-defined or otherwise named type, e.g. `JsonObject` or `Vec<T>`.
    Named(String),
}

impl Type {
    pub fn is_integer(&self) -> bool {
        matches!(self, Type::Int8 | Type::Int16 | Type::Int32 | Type::Int64)
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Type::Float32 | Type::Float64)
    }

    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    pub fn is_void(&self) -> bool {
        matches!(self, Type::Void)
    }

    /// Whether this type denotes a pointer (opaque or typed).
    pub fn is_pointer(&self) -> bool {
        matches!(self, Type::Pointer | Type::PointerTo(_))
    }

    /// Whether a value of this type may be bound to a target annotated with
    /// `target`. Identical types are always assignable; numeric values may be
    /// widened or narrowed across integer/float widths, and a plain string may
    /// be used where a path is expected. Other types are only assignable to
    /// themselves.
    pub fn is_assignable_to(&self, target: &Type) -> bool {
        if self == target {
            return true;
        }
        match (self, target) {
            (Type::Int8 | Type::Int16 | Type::Int32 | Type::Int64, target)
                if target.is_integer() || target.is_float() =>
            {
                true
            }
            (Type::Float32 | Type::Float64, Type::Float32 | Type::Float64) => true,
            (Type::String, Type::Path) => true,
            (Type::PointerTo(inner), Type::PointerTo(target)) => {
                inner == target || target.is_pointer()
            }
            (Type::List(inner), Type::List(target)) => inner.is_assignable_to(target),
            (Type::PointerTo(_), Type::Pointer) => true,
            (Type::Pointer, Type::PointerTo(_)) => true,
            (Type::List(_), Type::Pointer) => true,
            (Type::Pointer, Type::List(_)) => true,
            (Type::List(_), Type::Named(name)) | (Type::Named(name), Type::List(_)) => {
                name == "List"
            }
            _ => false,
        }
    }

    /// The canonical spelling used in compiler messages.
    pub fn name(&self) -> String {
        self.to_string()
    }

    /// Replace generic parameters using `bindings`, returning a concrete type.
    pub fn substitute(&self, bindings: &HashMap<String, Type>) -> Type {
        match self {
            Type::Generic(name) => bindings.get(name).cloned().unwrap_or_else(|| self.clone()),
            _ => self.clone(),
        }
    }

    /// Whether the type still contains unresolved generic parameters.
    pub fn has_generic(&self) -> bool {
        matches!(self, Type::Generic(_))
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Void => f.write_str("Void"),
            Type::Boolean => f.write_str("Boolean"),
            Type::String => f.write_str("String"),
            Type::Path => f.write_str("Path"),
            Type::Int8 => f.write_str("Int8"),
            Type::Int16 => f.write_str("Int16"),
            Type::Int32 => f.write_str("Int32"),
            Type::Int64 => f.write_str("Int64"),
            Type::Float32 => f.write_str("Float32"),
            Type::Float64 => f.write_str("Float64"),
            Type::Pointer => f.write_str("Pointer"),
            Type::PointerTo(inner) => write!(f, "{inner}*"),
            Type::List(inner) => write!(f, "List<{inner}>"),
            Type::Generic(name) | Type::Named(name) => f.write_str(name),
        }
    }
}
