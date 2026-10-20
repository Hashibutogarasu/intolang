use ast::Program;
use errors::{Error, Errors, Result};
use std::collections::HashMap;

use crate::context::EnumValues;
use crate::diagnostic::diagnostic;

/// Collect every `enum` of `program` with its variants' discriminants,
/// validating names and discriminants.
pub fn collect_enums(program: &Program, source_path: &str) -> Result<EnumValues> {
    let mut enums = EnumValues::new();
    for decl in program.enums() {
        let fail =
            |error: Errors| -> Error { diagnostic(source_path, decl.line, "module", error.into()) };
        if !decl.name.chars().next().is_some_and(char::is_uppercase) {
            return Err(fail(Errors::EnumNameCase));
        }
        let mut variants = HashMap::new();
        let mut next = 0i32;
        for variant in &decl.variants {
            let value = match variant.value {
                Some(value) => {
                    i32::try_from(value).map_err(|_| fail(Errors::EnumDiscriminantRange))?
                }
                None => next,
            };
            if !variant
                .name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase() || c == '_')
            {
                return Err(fail(Errors::EnumVariantNameCase));
            }
            if variants.insert(variant.name.clone(), value).is_some() {
                return Err(fail(Errors::DuplicateEnumVariant {
                    name: variant.name.clone(),
                }));
            }
            next = value.saturating_add(1);
        }
        if enums.insert(decl.name.clone(), variants).is_some() {
            return Err(fail(Errors::DuplicateEnum {
                name: decl.name.clone(),
            }));
        }
    }
    Ok(enums)
}
