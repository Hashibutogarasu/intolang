use ast::{FunctionDecl, Program};

use crate::context::UserFunction;

/// Collect the Into-bodied, non-generic functions of `program`.
///
/// Functions declared inside a class body are methods of that class; their
/// stored name is `Class.method`.
pub fn collect_user_functions(program: &Program) -> Vec<UserFunction> {
    let mut functions = Vec::new();
    for function in program.functions() {
        functions.extend(user_function(function, None));
    }
    for class in program.classes() {
        for method in &class.methods {
            functions.extend(user_function(method, Some(&class.name)));
        }
    }
    functions
}

fn user_function(function: &FunctionDecl, class: Option<&str>) -> Option<UserFunction> {
    if !function.type_params.is_empty() {
        return None;
    }
    let body = function.body.clone()?;
    Some(UserFunction {
        name: match class {
            Some(class) => format!("{class}.{}", function.name),
            None => function.name.clone(),
        },
        receiver: class.map(str::to_owned),
        is_static: function.is_static,
        params: function.params.clone(),
        body,
    })
}
