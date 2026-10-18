use ast::Program;

use crate::context::{ClassFields, FieldDef};

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
