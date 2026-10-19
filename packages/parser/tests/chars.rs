use parser::{CharLexeme, LiteralLexeme, TypeLexeme};
use types::{Literal, Type};

#[test]
fn decodes_character_bodies() {
    assert_eq!("A".parse::<CharLexeme>().unwrap().value(), 'A');
    assert_eq!("\\n".parse::<CharLexeme>().unwrap().value(), '\n');
    assert_eq!("\\'".parse::<CharLexeme>().unwrap().value(), '\'');
    assert_eq!("\\\\".parse::<CharLexeme>().unwrap().value(), '\\');
}

#[test]
fn a_character_body_holds_exactly_one_character() {
    assert!("".parse::<CharLexeme>().is_err());
    assert!("AB".parse::<CharLexeme>().is_err());
}

#[test]
fn a_quoted_character_is_a_literal() {
    let literal: Literal = "'A'".parse::<LiteralLexeme>().unwrap().into();
    assert_eq!(literal, Literal::Char('A'));
}

#[test]
fn char_is_a_type() {
    let ty: Type = "Char".parse::<TypeLexeme>().unwrap().into();
    assert_eq!(ty, Type::Char);
    assert_eq!(ty.to_string(), "Char");
}
