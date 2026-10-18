use ast::Program;
use std::collections::HashSet;

/// The generic parameter names declared by any function of `program`.
pub fn collect_type_params(program: &Program) -> HashSet<String> {
    let methods = program.classes().flat_map(|class| class.methods.iter());
    program
        .functions()
        .chain(methods)
        .flat_map(|function| function.type_params.iter().cloned())
        .collect()
}
