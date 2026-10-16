use anyhow::{Result, bail};
use std::collections::{HashMap, HashSet};

/// An explicit Into type as written in source, mirroring Rust's primitive set.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Void,
    Boolean,
    String,
    Int8,
    Int16,
    Int32,
    Int64,
    Float32,
    Float64,
    Pointer,
    /// A generic type parameter such as `T`.
    Generic(String),
    /// A user-defined or otherwise named type, e.g. `JsonObject` or `Vec<T>`.
    Named(String),
}

impl Type {
    /// Parse a type from source text, resolving bare identifiers to generic
    /// parameters when they are in scope.
    pub fn parse(text: &str, params: &HashSet<String>) -> Result<Type> {
        let text = text.trim();
        if text.is_empty() {
            bail!("Expected a type name");
        }
        Ok(match text {
            "void" | "Void" => Type::Void,
            "bool" | "Boolean" | "Bool" => Type::Boolean,
            "string" | "String" | "str" => Type::String,
            "i8" | "Int8" => Type::Int8,
            "i16" | "Int16" => Type::Int16,
            "i32" | "Int32" => Type::Int32,
            "i64" | "Int64" => Type::Int64,
            "int" | "Int" => Type::Int32,
            "f32" | "Float32" => Type::Float32,
            "f64" | "Float64" | "float" | "Float" | "double" => Type::Float64,
            "Pointer" | "pointer" | "Ptr" => Type::Pointer,
            _ => {
                if params.contains(text) {
                    Type::Generic(text.to_owned())
                } else {
                    Type::Named(normalize_named(text))
                }
            }
        })
    }

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

    /// The canonical spelling used in compiler messages.
    pub fn name(&self) -> String {
        match self {
            Type::Void => "Void".into(),
            Type::Boolean => "Boolean".into(),
            Type::String => "String".into(),
            Type::Int8 => "Int8".into(),
            Type::Int16 => "Int16".into(),
            Type::Int32 => "Int32".into(),
            Type::Int64 => "Int64".into(),
            Type::Float32 => "Float32".into(),
            Type::Float64 => "Float64".into(),
            Type::Pointer => "Pointer".into(),
            Type::Generic(name) | Type::Named(name) => name.clone(),
        }
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

fn normalize_named(text: &str) -> String {
    let text = text.trim();
    match text.split_once('<') {
        Some((base, rest)) => {
            let args = rest.trim_end_matches('>');
            format!("{}<{}>", base.trim(), args.trim())
        }
        None => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_primitives() {
        let params = HashSet::new();
        assert_eq!(Type::parse("String", &params).unwrap(), Type::String);
        assert_eq!(Type::parse("Int32", &params).unwrap(), Type::Int32);
        assert_eq!(Type::parse("Int64", &params).unwrap(), Type::Int64);
        assert_eq!(Type::parse("Float", &params).unwrap(), Type::Float64);
        assert_eq!(Type::parse("Float64", &params).unwrap(), Type::Float64);
    }

    #[test]
    fn resolves_generic_parameters() {
        let params = HashSet::from(["T".to_owned()]);
        assert_eq!(
            Type::parse("T", &params).unwrap(),
            Type::Generic("T".into())
        );
        assert_eq!(
            Type::parse("T", &HashSet::new()).unwrap(),
            Type::Named("T".into())
        );
        assert_eq!(
            Type::parse("Vec<T>", &params).unwrap(),
            Type::Named("Vec<T>".into())
        );
    }

    #[test]
    fn substitutes_generics() {
        let bindings = HashMap::from([("T".to_owned(), Type::Int64)]);
        let ty = Type::Generic("T".into());
        assert_eq!(ty.substitute(&bindings), Type::Int64);
        assert_eq!(Type::Int32.substitute(&bindings), Type::Int32);
    }
}
