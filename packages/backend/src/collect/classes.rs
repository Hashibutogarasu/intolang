use ast::Program;

use crate::context::{ClassFields, FieldDef};

/// The attribute that makes a class read the members it does not declare with
/// its `get` method.
const DYNAMIC: &str = "dynamic";

/// The names of the classes of `program` marked `#[dynamic]`.
pub fn collect_dynamic_classes(program: &Program) -> Vec<String> {
    program
        .classes()
        .filter(|class| {
            class
                .attributes
                .iter()
                .any(|attribute| attribute.name == DYNAMIC)
        })
        .map(|class| class.name.clone())
        .collect()
}

/// Collect the declared fields of every class in `program`.
pub fn collect_classes(program: &Program) -> ClassFields {
    program
        .classes()
        .map(|class| {
            let fields = class
                .fields
                .iter()
                .map(|field| FieldDef {
                    name: field.name.clone(),
                    ty: field.ty.to_string(),
                    default: field.default.clone(),
                    is_static: field.is_static,
                })
                .collect();
            (class.name.clone(), fields)
        })
        .collect()
}
