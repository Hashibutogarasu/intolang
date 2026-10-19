use std::collections::HashMap;
use std::fmt;

/// An explicit Into type as written in source, mirroring Rust's primitive set.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Void,
    Boolean,
    String,
    Path,
    Char,
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
    /// A reference that owns nothing, written `T&`.
    Reference(Box<Type>),
    /// A generic type parameter such as `T`.
    Generic(String),
    /// A user-defined or otherwise named type, e.g. `JsonObject`.
    Named(String),
    /// A generic class given its type arguments, e.g. `Callback<JsonField>`.
    Applied(String, Vec<Type>),
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

    /// Whether this type is a reference (`T&`) rather than an owning value.
    pub fn is_reference(&self) -> bool {
        matches!(self, Type::Reference(_))
    }

    /// The type a reference refers to, or the type itself for a value.
    pub fn referent(&self) -> &Type {
        match self {
            Type::Reference(inner) => inner,
            other => other,
        }
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
            (Type::Reference(inner), Type::Reference(target)) => inner.is_assignable_to(target),
            (value, Type::Reference(target)) => value.is_assignable_to(target),
            (Type::Reference(_), _) => false,
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
            (Type::Applied(name, arguments), Type::Applied(target_name, target_arguments)) => {
                name == target_name
                    && arguments.len() == target_arguments.len()
                    && arguments
                        .iter()
                        .zip(target_arguments)
                        .all(|(argument, target)| target.accepts_argument(argument))
            }
            (Type::Applied(name, _), Type::Named(target))
            | (Type::Named(target), Type::Applied(name, _)) => name == target,
            _ => false,
        }
    }

    /// Whether `argument` may stand where this type argument is written: a
    /// generic parameter takes any type, and the types are otherwise equal or
    /// assignable.
    fn accepts_argument(&self, argument: &Type) -> bool {
        matches!(self, Type::Generic(_))
            || matches!(argument, Type::Generic(_))
            || argument.is_assignable_to(self)
    }

    /// The name of the class this type is an instance of, with the type
    /// arguments left out.
    pub fn class_name(&self) -> Option<&str> {
        match self {
            Type::Named(name) | Type::Applied(name, _) => Some(name),
            _ => None,
        }
    }

    /// The type arguments of a generic class type.
    pub fn arguments(&self) -> &[Type] {
        match self {
            Type::Applied(_, arguments) => arguments,
            _ => &[],
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
            Type::PointerTo(inner) => Type::PointerTo(Box::new(inner.substitute(bindings))),
            Type::List(inner) => Type::List(Box::new(inner.substitute(bindings))),
            Type::Reference(inner) => Type::Reference(Box::new(inner.substitute(bindings))),
            Type::Applied(name, arguments) => Type::Applied(
                name.clone(),
                arguments
                    .iter()
                    .map(|argument| argument.substitute(bindings))
                    .collect(),
            ),
            _ => self.clone(),
        }
    }

    /// Whether the type still contains unresolved generic parameters.
    pub fn has_generic(&self) -> bool {
        match self {
            Type::Generic(_) => true,
            Type::PointerTo(inner) | Type::List(inner) | Type::Reference(inner) => {
                inner.has_generic()
            }
            Type::Applied(_, arguments) => arguments.iter().any(Type::has_generic),
            _ => false,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Void => f.write_str("Void"),
            Type::Boolean => f.write_str("Boolean"),
            Type::String => f.write_str("String"),
            Type::Path => f.write_str("Path"),
            Type::Char => f.write_str("Char"),
            Type::Int8 => f.write_str("Int8"),
            Type::Int16 => f.write_str("Int16"),
            Type::Int32 => f.write_str("Int32"),
            Type::Int64 => f.write_str("Int64"),
            Type::Float32 => f.write_str("Float32"),
            Type::Float64 => f.write_str("Float64"),
            Type::Pointer => f.write_str("Pointer"),
            Type::PointerTo(inner) => write!(f, "{inner}*"),
            Type::List(inner) => write!(f, "List<{inner}>"),
            Type::Reference(inner) => write!(f, "{inner}&"),
            Type::Generic(name) | Type::Named(name) => f.write_str(name),
            Type::Applied(name, arguments) => {
                write!(f, "{name}<")?;
                for (index, argument) in arguments.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{argument}")?;
                }
                f.write_str(">")
            }
        }
    }
}
