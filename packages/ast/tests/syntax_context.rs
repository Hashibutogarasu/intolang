use ast::{Program, SyntaxError};
use errors::{Domain, Error, Errors, Layer, Severity};

fn bound(source: &str) -> Error {
    let error = source.parse::<Program>().unwrap_err();
    let bound = SyntaxError::bind(error, "main.into");
    bound
        .downcast::<Error>()
        .expect("the failure is an error of the Into language")
}

#[test]
fn a_syntax_error_names_what_was_expected_and_what_was_found() {
    let error = source_error("enum Format Text }");
    assert_eq!(error.line, 1);
    assert_eq!(error.expected, vec!["`{`".to_string()]);
    assert_eq!(error.found.as_deref(), Some("`Text`"));
    let span = error.span.expect("the span of the token that was found");
    assert_eq!((span.line, span.start), (1, 12));
}

#[test]
fn the_end_of_the_input_is_what_was_found_at_the_end() {
    let error = source_error("into x = [1, 2");
    assert_eq!(error.found.as_deref(), Some("end of input"));
    assert_eq!(error.expected, vec!["`]`".to_string()]);
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

fn source_error(source: &str) -> SyntaxError {
    source
        .parse::<Program>()
        .unwrap_err()
        .downcast::<SyntaxError>()
        .expect("a syntax error")
}
