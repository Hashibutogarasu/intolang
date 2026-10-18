use types::Type;

/// Check an inferred type against an explicit annotation, returning the type
/// the binding takes.
pub fn resolve_annotation(inferred: Type, annotation: Option<Type>) -> Result<Type, String> {
    match annotation {
        Some(target) => {
            if inferred.is_assignable_to(&target) {
                Ok(target)
            } else {
                Err(format!(
                    "Type mismatch: expected `{}` but found `{}`",
                    target.name(),
                    inferred.name()
                ))
            }
        }
        None => Ok(inferred),
    }
}
