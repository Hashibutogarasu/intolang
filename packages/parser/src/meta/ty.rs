use anyhow::{Error, Result, bail};
use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;
use types::Type;

/// A type as written in source. Bare identifiers resolve to generic
/// parameters only through [`TypeLexeme::with_generics`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeLexeme(Type);

impl TypeLexeme {
    pub fn with_generics(text: &str, params: &HashSet<String>) -> Result<Self> {
        let text = text.trim();
        if text.is_empty() {
            bail!("Expected a type name");
        }
        if let Some(inner) = text.strip_suffix('*') {
            let inner = inner.trim();
            if inner.is_empty() {
                bail!("Pointer type `*` needs a pointee type");
            }
            let inner = Self::with_generics(inner, params)?;
            return Ok(Self(Type::PointerTo(Box::new(inner.0))));
        }
        if let Some(inner) = text
            .strip_prefix("List<")
            .and_then(|inner| inner.strip_suffix('>'))
        {
            let inner = inner.trim();
            if inner.is_empty() {
                bail!("List type `List<>` needs an element type");
            }
            let inner = Self::with_generics(inner, params)?;
            return Ok(Self(Type::List(Box::new(inner.0))));
        }
        Ok(Self(match text {
            "void" | "Void" => Type::Void,
            "bool" | "Boolean" | "Bool" => Type::Boolean,
            "string" | "String" | "str" => Type::String,
            "path" | "Path" => Type::Path,
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
        }))
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

impl FromStr for TypeLexeme {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        Self::with_generics(text, &HashSet::new())
    }
}

impl TryFrom<&str> for TypeLexeme {
    type Error = Error;

    fn try_from(text: &str) -> Result<Self> {
        text.parse()
    }
}

impl From<Type> for TypeLexeme {
    fn from(ty: Type) -> Self {
        Self(ty)
    }
}

impl From<TypeLexeme> for Type {
    fn from(lexeme: TypeLexeme) -> Self {
        lexeme.0
    }
}

impl fmt::Display for TypeLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
