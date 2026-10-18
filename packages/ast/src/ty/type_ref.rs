use anyhow::Result;
use parser::TypeLexeme;
use std::collections::HashSet;
use std::fmt;
use types::{Punct, Type};

use crate::Cursor;

/// A type exactly as written, in canonical spelling (`List<Int32>*`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeRef(String);

impl TypeRef {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Resolve the written type, treating names in `params` as generics.
    pub fn resolve(&self, params: &HashSet<String>) -> Result<Type> {
        Ok(TypeLexeme::with_generics(&self.0, params)?.into())
    }
}

impl From<&str> for TypeRef {
    fn from(text: &str) -> Self {
        Self(text.to_owned())
    }
}

impl TryFrom<&mut Cursor<'_>> for TypeRef {
    type Error = anyhow::Error;

    fn try_from(cursor: &mut Cursor<'_>) -> Result<Self> {
        let (name, _) = cursor.expect_ident("a type name")?;
        let mut text = name;
        if cursor.peek_symbol(0) == Some('<') {
            cursor.bump();
            text.push('<');
            let mut first = true;
            loop {
                if !first {
                    if cursor.eat_punct(Punct::Comma) {
                        text.push_str(", ");
                    } else {
                        break;
                    }
                }
                first = false;
                let argument = TypeRef::try_from(&mut *cursor)?;
                text.push_str(&argument.0);
            }
            if cursor.peek_symbol(0) != Some('>') {
                return Err(cursor.error("Expected `>` to close the type arguments"));
            }
            cursor.bump();
            text.push('>');
        }
        while cursor.peek_symbol(0) == Some('*') {
            cursor.bump();
            text.push('*');
        }
        if cursor.peek_symbol(0) == Some('&') {
            cursor.bump();
            text.push('&');
        }
        Ok(Self(text))
    }
}

impl fmt::Display for TypeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
