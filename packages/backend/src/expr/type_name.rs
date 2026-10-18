use anyhow::Result;
use parser::TypeLexeme;
use std::collections::HashSet;
use types::Type;

/// Parse a type spelling, treating names in `params` as generic parameters.
pub fn parse_type(text: &str, params: &HashSet<String>) -> Result<Type> {
    Ok(TypeLexeme::with_generics(text, params)?.into())
}

/// The class name underlying a (possibly generic) type spelling, e.g.
/// `List<Int32>` becomes `List`, and a reference `Process&` becomes `Process`.
/// Non-generic names are returned unchanged.
pub fn base_type_name(name: &str) -> &str {
    name.split('<')
        .next()
        .unwrap_or(name)
        .trim()
        .trim_end_matches('&')
}
