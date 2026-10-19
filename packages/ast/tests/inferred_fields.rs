use ast::{Program, SyntaxError};

fn class(source: &str) -> ast::ClassDecl {
    let program: Program = source.parse().unwrap();
    program.classes().next().unwrap().clone()
}

#[test]
fn a_field_takes_the_type_of_its_literal() {
    let cat = class(
        "pub Cat(): JsonObject {\n    pub into neko = \"cat\";\n    pub into lives = 9;\n    pub into weight = 4.5;\n    pub into tame = true;\n    pub into mark = 'c';\n}\n",
    );
    let types: Vec<&str> = cat.fields.iter().map(|field| field.ty.as_str()).collect();
    assert_eq!(types, vec!["String", "Int32", "Float64", "Boolean", "Char"]);
    assert_eq!(cat.fields[0].name, "neko");
    assert!(cat.fields[0].default.is_some());
}

#[test]
fn a_field_that_starts_as_a_class_takes_that_class() {
    let dog = class(
        "pub Dog(): JsonObject {\n    pub into foo = \"bar\";\n    pub into cat = Cat();\n}\n",
    );
    assert_eq!(dog.fields[1].ty.as_str(), "Cat");
}

#[test]
fn a_static_field_may_infer_its_type_too() {
    let config = class("pub Config() {\n    pub static into LIMIT = 5;\n}\n");
    assert!(config.fields[0].is_static);
    assert_eq!(config.fields[0].ty.as_str(), "Int32");
}

#[test]
fn the_written_type_still_works() {
    let config = class("pub Config() {\n    pub into: Int64 big = 5;\n}\n");
    assert_eq!(config.fields[0].ty.as_str(), "Int64");
}

#[test]
fn a_field_needs_a_type_or_a_value() {
    let error = "pub Bad() {\n    pub into nothing;\n}\n"
        .parse::<Program>()
        .unwrap_err();
    let syntax = error.downcast_ref::<SyntaxError>().unwrap();
    assert!(
        syntax.message.contains("needs a type or a value"),
        "{}",
        syntax.message
    );
}

#[test]
fn a_value_that_has_no_obvious_type_is_rejected() {
    let error = "pub Bad() {\n    pub into total = a + b;\n}\n"
        .parse::<Program>()
        .unwrap_err();
    let syntax = error.downcast_ref::<SyntaxError>().unwrap();
    assert!(
        syntax.message.contains("cannot be inferred"),
        "{}",
        syntax.message
    );
}
