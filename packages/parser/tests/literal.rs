use parser::{LiteralLexeme, StringBody};
use types::Literal;

fn literal(text: &str) -> Option<Literal> {
    text.parse::<LiteralLexeme>().ok().map(Into::into)
}

#[test]
fn parses_literals() {
    assert_eq!(literal("42"), Some(Literal::Int(42)));
    assert_eq!(literal("-7"), Some(Literal::Int(-7)));
    assert_eq!(literal("1.5"), Some(Literal::Float(1.5)));
    assert_eq!(literal("true"), Some(Literal::Bool(true)));
    assert_eq!(literal("false"), Some(Literal::Bool(false)));
    assert_eq!(
        literal("\"a\\nb\""),
        Some(Literal::String("a\nb".to_owned()))
    );
    assert_eq!(literal("name"), None);
}

#[test]
fn decodes_string_bodies() {
    assert_eq!(StringBody::from("a\\tb\\r\\\\").as_str(), "a\tb\r\\");
    assert_eq!(StringBody::from("\\q").as_str(), "\\q");
    assert_eq!(StringBody::from("end\\").as_str(), "end\\");
}
