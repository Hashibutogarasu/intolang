use parser::TypeLexeme;
use std::collections::HashSet;
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
fn parses_reference_types() {
    let process = Type::Named("Process".into());
    assert_eq!(
        parse("Process&"),
        Type::Reference(Box::new(process.clone()))
    );
    assert_eq!(parse("Process&").name(), "Process&");
    assert_eq!(
        parse("Int32*&"),
        Type::Reference(Box::new(Type::PointerTo(Box::new(Type::Int32))))
    );
    assert!(parse("Process&").is_reference());
    assert_eq!(parse("Process&").referent(), &process);
}

#[test]
fn parses_primitives_and_aliases() {
    assert_eq!(parse("String"), Type::String);
    assert_eq!(parse("str"), Type::String);
    assert_eq!(parse("Path"), Type::Path);
    assert_eq!(parse("Int32"), Type::Int32);
    assert_eq!(parse("i32"), Type::Int32);
    assert_eq!(parse("Int"), Type::Int32);
    assert_eq!(parse("Int64"), Type::Int64);
    assert_eq!(parse("Float"), Type::Float64);
    assert_eq!(parse("Float64"), Type::Float64);
    assert_eq!(parse("Ptr"), Type::Pointer);
}

#[test]
fn resolves_generic_parameters() {
    let params = HashSet::from(["T".to_owned()]);
    assert_eq!(parse_with("T", &params), Type::Generic("T".into()));
    assert_eq!(parse("T"), Type::Named("T".into()));
    assert_eq!(parse_with("Vec<T>", &params), Type::Named("Vec<T>".into()));
}

#[test]
fn normalizes_named_generic_spelling() {
    assert_eq!(parse("Map< A , B >"), Type::Named("Map<A , B>".into()));
}

#[test]
fn rejects_empty_types() {
    assert!("".parse::<TypeLexeme>().is_err());
    assert!("*".parse::<TypeLexeme>().is_err());
    assert!("&".parse::<TypeLexeme>().is_err());
    assert!("List<>".parse::<TypeLexeme>().is_err());
}
