use ast::{FunctionDecl, Program, overload_key};
use std::collections::HashMap;

use crate::context::{Ownership, UserFunction};

const CONSUMES: &str = "consumes";
const DETACHED: &str = "detached";
const INSPECTS: &str = "inspects";
pub const CAST_EVENT: &str = "onCast";

/// Collect the Into-bodied, non-generic functions of `program`.
///
/// Functions declared inside a class body are methods of that class; their
/// stored name is `Class.method`.
pub fn collect_user_functions(program: &Program) -> Vec<UserFunction> {
    let mut functions = Vec::new();
    let top_level: Vec<&FunctionDecl> = program.functions().collect();
    for function in &top_level {
        functions.extend(user_function(function, None, &top_level));
    }
    for class in program.classes() {
        let methods: Vec<&FunctionDecl> = class.methods.iter().collect();
        for method in &methods {
            functions.extend(user_function(method, Some(&class.name), &methods));
        }
    }
    functions
}

/// The name `Class.onCast<From,To>` under which a cast handler of a class is
/// stored and looked up.
pub fn cast_handler_key(class: &str, from: &str, to: &str) -> String {
    format!("{class}.{CAST_EVENT}<{from},{to}>")
}

/// Collect the `onCast` events declared in the classes of `program` as
/// functions without a receiver, one for each pair of type arguments.
pub fn collect_cast_handlers(program: &Program) -> Vec<UserFunction> {
    let mut handlers = Vec::new();
    for class in program.classes() {
        for event in class.events.iter().filter(|event| event.name == CAST_EVENT) {
            let [from, to] = event.type_params.as_slice() else {
                continue;
            };
            handlers.push(UserFunction {
                name: cast_handler_key(&class.name, from, to),
                base: cast_handler_key(&class.name, from, to),
                receiver: None,
                is_static: true,
                params: event.params.clone(),
                return_type: None,
                body: event.body.clone(),
                inspects: false,
            });
        }
    }
    handlers
}

fn user_function(
    function: &FunctionDecl,
    class: Option<&str>,
    siblings: &[&FunctionDecl],
) -> Option<UserFunction> {
    if !function.type_params.is_empty() {
        return None;
    }
    let body = function.body.clone()?;
    let base = match class {
        Some(class) => format!("{class}.{}", function.name),
        None => function.name.clone(),
    };
    let shared = siblings
        .iter()
        .filter(|other| other.name == function.name)
        .count();
    Some(UserFunction {
        name: if shared > 1 {
            overload_key(&base, &function.params)
        } else {
            base.clone()
        },
        base,
        receiver: class.map(str::to_owned),
        is_static: function.is_static,
        params: function.params.clone(),
        return_type: function.return_type.as_ref().map(ToString::to_string),
        body,
        inspects: function
            .attributes
            .iter()
            .any(|attribute| attribute.name == INSPECTS),
    })
}

pub fn collect_consuming(program: &Program) -> HashMap<String, Ownership> {
    program
        .classes()
        .flat_map(|class| {
            class.methods.iter().filter_map(|method| {
                let attribute = method
                    .attributes
                    .iter()
                    .find(|attribute| attribute.name == CONSUMES)?;
                let state = match attribute.string_args().as_deref() {
                    Some([DETACHED]) => Ownership::Detached,
                    _ => Ownership::Moved,
                };
                Some((format!("{}.{}", class.name, method.name), state))
            })
        })
        .collect()
}
