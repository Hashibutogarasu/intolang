use ast::Program;
use errors::{Domain, Error, Errors, Layer, Severity};

fn failure(source: &str) -> Error {
    source.parse::<Program>().unwrap_err()
}

fn bound(source: &str) -> Error {
    failure(source).bound_to("main.into")
}

#[test]
fn a_syntax_error_names_what_was_expected_and_what_was_found() {
    let context = failure("enum Format Text }").resolved_context();
    let span = context.span.expect("the span of the token that was found");
    assert_eq!((span.line, span.start), (1, 12));
    let syntax = context.syntax.expect("a syntax error has a syntax context");
    assert_eq!(syntax.expected, vec!["`{`".to_string()]);
    assert_eq!(syntax.found.as_deref(), Some("`Text`"));
}

#[test]
fn the_end_of_the_input_is_what_was_found_at_the_end() {
    let syntax = failure("into x = [1, 2")
        .resolved_context()
        .syntax
        .expect("a syntax error has a syntax context");
    assert_eq!(syntax.found.as_deref(), Some("end of input"));
    assert_eq!(syntax.expected, vec!["`]`".to_string()]);
}

#[test]
fn a_syntax_error_has_no_path_before_it_is_bound() {
    let error = failure("enum Format Text }");
    assert!(error.trace().into.is_none());
    assert!(error.resolved_context().path.is_none());
}

#[test]
fn a_bound_syntax_error_carries_its_context_as_data() {
    let error = bound("enum Format Text }");
    assert_eq!(error.code(), Some(0x0001));
    let context = error.resolved_context();
    assert_eq!(context.domain, Domain::IntoLang);
    assert_eq!(context.severity, Severity::Error);
    assert_eq!(context.path.as_deref(), Some("main.into"));
    assert_eq!(context.span.unwrap().line, 1);
    let syntax = context.syntax.expect("a syntax error has a syntax context");
    assert_eq!(syntax.scope, "module");
    assert_eq!(syntax.expected, vec!["`{`".to_string()]);
    assert_eq!(syntax.found.as_deref(), Some("`Text`"));

    let layers = error.trace().layers();
    assert!(layers.iter().any(|layer| layer.layer() == Layer::Into));
    assert!(error.to_string().contains("module at main.into:1"));
}

#[test]
fn binding_twice_keeps_the_first_path() {
    let error = bound("enum Format Text }").bound_to("other.into");
    assert_eq!(error.resolved_context().path.as_deref(), Some("main.into"));
}

#[test]
fn an_unterminated_string_is_bound_to_its_path_and_line() {
    let error = bound("into a = 1;\ninto b = \"abc;");
    assert_eq!(error.kind(), &Errors::UnterminatedString { line: 2 });
    assert_eq!(error.code(), Some(0x0008));
    let context = error.resolved_context();
    assert_eq!(context.path.as_deref(), Some("main.into"));
    assert_eq!(context.span.unwrap().line, 2);
    assert!(error.to_string().contains("main.into:2"));
}

#[test]
fn an_unterminated_character_literal_has_its_own_code() {
    let error = bound("into c = 'a;");
    assert_eq!(error.kind(), &Errors::UnterminatedChar { line: 1 });
    assert_eq!(error.code(), Some(0x0009));
}
