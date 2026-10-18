use anyhow::Result;
use ast::Program;
use std::collections::HashMap;

use crate::context::EnumValues;
use crate::diagnostic::diagnostic;

/// Collect every `enum` of `program` with its variants' discriminants,
/// validating names and discriminants.
pub fn collect_enums(program: &Program, source_path: &str) -> Result<EnumValues> {
    let mut enums = EnumValues::new();
    for decl in program.enums() {
        let fail = |message: &str| diagnostic(source_path, decl.line, "module", message);
        if !decl.name.chars().next().is_some_and(char::is_uppercase) {
            return Err(fail("Enum names must start with an uppercase letter"));
        }
        let mut variants = HashMap::new();
        let mut next = 0i32;
        for variant in &decl.variants {
            let value = match variant.value {
                Some(value) => i32::try_from(value)
                    .map_err(|_| fail("Enum discriminants must be 32-bit integers"))?,
                None => next,
            };
            if !variant
                .name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase() || c == '_')
            {
                return Err(fail("Enum variant names must start with an uppercase letter"));
            }
            if variants.insert(variant.name.clone(), value).is_some() {
                return Err(fail(&format!("Duplicate enum variant `{}`", variant.name)));
            }
            next = value.saturating_add(1);
        }
        if enums.insert(decl.name.clone(), variants).is_some() {
            return Err(fail(&format!("Duplicate enum `{}`", decl.name)));
        }
    }
    Ok(enums)
}
