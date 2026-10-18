use parser::TypeLexeme;
use std::collections::{HashMap, HashSet};
use types::Type;

fn parse(text: &str) -> Type {
    text.parse::<TypeLexeme>().unwrap().into()
}

fn parse_with(text: &str, params: &HashSet<String>) -> Type {
    TypeLexeme::with_generics(text, params).unwrap().into()
}

#[test]
fn parses_pointer_types() {
    assert_eq!(parse("String*"), Type::PointerTo(Box::new(Type::String)));
    assert_eq!(parse("Int32*"), Type::PointerTo(Box::new(Type::Int32)));
    assert_eq!(
        parse("List<Int32>*"),
        Type::PointerTo(Box::new(Type::List(Box::new(Type::Int32))))
    );
    assert_eq!(parse("List<String>"), Type::List(Box::new(Type::String)));
    assert_eq!(parse("List<String>").name(), "List<String>");
    assert_eq!(parse("String*").name(), "String*");
    assert!(parse("String*").is_pointer());
}

#[test]
fn parses_primitives() {
    assert_eq!(parse("String"), Type::String);
    assert_eq!(parse("Path"), Type::Path);
    assert_eq!(parse("Int32"), Type::Int32);
    assert_eq!(parse("Int64"), Type::Int64);
    assert_eq!(parse("Float"), Type::Float64);
    assert_eq!(parse("Float64"), Type::Float64);
}

#[test]
fn resolves_generic_parameters() {
    let params = HashSet::from(["T".to_owned()]);
    assert_eq!(parse_with("T", &params), Type::Generic("T".into()));
    assert_eq!(parse("T"), Type::Named("T".into()));
    assert_eq!(parse_with("Vec<T>", &params), Type::Named("Vec<T>".into()));
}

#[test]
fn rejects_empty_types() {
    assert!("".parse::<TypeLexeme>().is_err());
    assert!("*".parse::<TypeLexeme>().is_err());
    assert!("List<>".parse::<TypeLexeme>().is_err());
}

#[test]
fn substitutes_generics() {
    let bindings = HashMap::from([("T".to_owned(), Type::Int64)]);
    let ty = Type::Generic("T".into());
    assert_eq!(ty.substitute(&bindings), Type::Int64);
    assert_eq!(Type::Int32.substitute(&bindings), Type::Int32);
}

#[test]
fn checks_assignability() {
    assert!(Type::Int32.is_assignable_to(&Type::Int32));
    assert!(Type::Int32.is_assignable_to(&Type::Int64));
    assert!(Type::Int32.is_assignable_to(&Type::Float64));
    assert!(Type::Float64.is_assignable_to(&Type::Float32));
    assert!(Type::String.is_assignable_to(&Type::Path));
    assert!(!Type::Path.is_assignable_to(&Type::String));
    assert!(!Type::String.is_assignable_to(&Type::Int32));
    assert!(!Type::Int32.is_assignable_to(&Type::String));
    assert!(!Type::Boolean.is_assignable_to(&Type::Int32));
    assert!(!Type::Float64.is_assignable_to(&Type::Int32));
    assert!(!Type::Named("JsonObject".into()).is_assignable_to(&Type::Pointer));
}
