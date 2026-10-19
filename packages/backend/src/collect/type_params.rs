use ast::Program;
use std::collections::HashSet;

/// The generic parameter names declared by any function or class of
/// `program`.
pub fn collect_type_params(program: &Program) -> HashSet<String> {
    let methods = program.classes().flat_map(|class| class.methods.iter());
    let functions = program
        .functions()
        .chain(methods)
        .flat_map(|function| function.type_params.iter().cloned());
    let classes = program
        .classes()
        .flat_map(|class| class.type_params.iter().cloned());
    functions.chain(classes).collect()
}
