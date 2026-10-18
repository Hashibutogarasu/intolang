use ast::Program;
use std::collections::HashSet;

/// Collect the names a module exposes publicly.
///
/// Only declarations carrying the `pub` modifier are visible to importers.
/// Free declarations contribute their bare name and class members contribute
/// `Class.member`; `pub` classes and `pub macro`s are included as well.
pub fn public_items(program: &Program) -> HashSet<String> {
    let mut items = HashSet::new();
    for class in program.classes() {
        if class.is_public {
            items.insert(class.name.clone());
        }
        for method in class.methods.iter().filter(|method| method.is_public) {
            items.insert(format!("{}.{}", class.name, method.name));
        }
    }
    for decl in program.macros().filter(|decl| decl.is_public) {
        items.insert(decl.name.clone());
    }
    for function in program.functions().filter(|function| function.is_public) {
        items.insert(function.name.clone());
    }
    items
}
