use ast::Expr;
use std::collections::HashMap;
use types::Literal;

/// A declared class field: `pub into: <Type> <name> = <default>;`.
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub ty: String,
    pub default: Option<Expr>,
    pub is_static: bool,
}

impl FieldDef {
    /// The default value when it is a literal.
    pub fn default_literal(&self) -> Option<&Literal> {
        match self.default.as_ref()? {
            Expr::Literal(literal) => Some(literal),
            _ => None,
        }
    }
}

pub type ClassFields = HashMap<String, Vec<FieldDef>>;

/// Enum name to variant name to discriminant.
pub type EnumValues = HashMap<String, HashMap<String, i32>>;
