use std::collections::HashMap;
use types::Type;

fn pointer_to(ty: Type) -> Type {
    Type::PointerTo(Box::new(ty))
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

#[test]
fn pointers_and_lists_convert_to_the_opaque_pointer() {
    assert!(pointer_to(Type::Int32).is_assignable_to(&Type::Pointer));
    assert!(Type::Pointer.is_assignable_to(&pointer_to(Type::Int32)));
    assert!(Type::List(Box::new(Type::Int32)).is_assignable_to(&Type::Pointer));
    assert!(pointer_to(Type::Int32).is_assignable_to(&pointer_to(Type::Pointer)));
    assert!(!pointer_to(Type::Int32).is_assignable_to(&pointer_to(Type::String)));
}

#[test]
fn substitutes_generics() {
    let bindings = HashMap::from([("T".to_owned(), Type::Int64)]);
    let ty = Type::Generic("T".into());
    assert!(ty.has_generic());
    assert_eq!(ty.substitute(&bindings), Type::Int64);
    assert_eq!(Type::Int32.substitute(&bindings), Type::Int32);
}

#[test]
fn classifies_types() {
    assert!(Type::Int8.is_integer() && Type::Int8.is_numeric());
    assert!(Type::Float32.is_float() && Type::Float32.is_numeric());
    assert!(!Type::String.is_numeric());
    assert!(Type::Void.is_void());
    assert!(Type::Pointer.is_pointer() && pointer_to(Type::Int8).is_pointer());
}

#[test]
fn displays_the_canonical_spelling() {
    assert_eq!(Type::Int32.to_string(), "Int32");
    assert_eq!(pointer_to(Type::String).to_string(), "String*");
    assert_eq!(
        Type::List(Box::new(Type::Named("Vec<T>".into()))).name(),
        "List<Vec<T>>"
    );
}
