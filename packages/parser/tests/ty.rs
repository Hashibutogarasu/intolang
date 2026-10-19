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
    assert_eq!(
        parse_with("Vec<T>", &params),
        Type::Applied("Vec".into(), vec![Type::Generic("T".into())])
    );
}

#[test]
fn parses_generic_classes_with_their_arguments() {
    assert_eq!(
        parse("Map< A , B >"),
        Type::Applied(
            "Map".into(),
            vec![Type::Named("A".into()), Type::Named("B".into())]
        )
    );
    assert_eq!(parse("Map< A , B >").name(), "Map<A, B>");
    assert_eq!(
        parse("Json<String, Object>"),
        Type::Applied(
            "Json".into(),
            vec![Type::String, Type::Named("Object".into())]
        )
    );
    assert_eq!(
        parse("Callback<List<Int32>>"),
        Type::Applied("Callback".into(), vec![Type::List(Box::new(Type::Int32))])
    );
    assert_eq!(
        parse("Pair<Box<Int32>, String>"),
        Type::Applied(
            "Pair".into(),
            vec![Type::Applied("Box".into(), vec![Type::Int32]), Type::String]
        )
    );
    assert_eq!(parse("Box<Int32>&").referent().class_name(), Some("Box"));
}

#[test]
fn applied_types_compare_through_their_arguments() {
    let callback = |argument: Type| Type::Applied("Callback".into(), vec![argument]);
    assert!(callback(Type::Int32).is_assignable_to(&callback(Type::Int32)));
    assert!(callback(Type::Int32).is_assignable_to(&callback(Type::Int64)));
    assert!(!callback(Type::String).is_assignable_to(&callback(Type::Int32)));
    assert!(callback(Type::String).is_assignable_to(&callback(Type::Generic("T".into()))));
    assert!(callback(Type::String).is_assignable_to(&Type::Named("Callback".into())));
    assert!(
        !callback(Type::String)
            .is_assignable_to(&Type::Applied("Event".into(), vec![Type::String]))
    );
    assert!(!callback(Type::String).is_assignable_to(&Type::Applied(
        "Callback".into(),
        vec![Type::String, Type::String]
    )));
}

#[test]
fn generic_parameters_are_replaced_inside_applied_types() {
    let bindings = std::collections::HashMap::from([("T".to_owned(), Type::Int32)]);
    let written = Type::Reference(Box::new(Type::Applied(
        "Box".into(),
        vec![Type::List(Box::new(Type::Generic("T".into())))],
    )));
    assert!(written.has_generic());
    let resolved = written.substitute(&bindings);
    assert!(!resolved.has_generic());
    assert_eq!(resolved.name(), "Box<List<Int32>>&");
}

#[test]
fn rejects_empty_types() {
    assert!("".parse::<TypeLexeme>().is_err());
    assert!("*".parse::<TypeLexeme>().is_err());
    assert!("&".parse::<TypeLexeme>().is_err());
    assert!("List<>".parse::<TypeLexeme>().is_err());
}
