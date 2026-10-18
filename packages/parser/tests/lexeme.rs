use parser::{
    CommentLexeme, ComparisonLexeme, IdentLexeme, LanguageLexeme, NumberLexeme, PathLexeme,
    StringLexeme, SymbolLexeme, TagLexeme,
};
use std::path::Path;
use types::{ComparisonOperator, Language, MessageTag, Token, TokenKind};

#[test]
fn converts_lexemes_into_tokens() {
    let ident: Token = "intoVar".parse::<IdentLexeme>().unwrap().at_line(3).into();
    assert_eq!(
        ident,
        Token::new(TokenKind::Identifier("intoVar".into()), 3)
    );
    let number: Token = "42".parse::<NumberLexeme>().unwrap().into();
    assert_eq!(number.kind, TokenKind::Number("42".into()));
    let symbol: Token = SymbolLexeme::from('{').at_line(2).into();
    assert_eq!(symbol.kind, TokenKind::Symbol('{'));
    assert_eq!(symbol.line(), 2);
    let comment: Token = "// note".parse::<CommentLexeme>().unwrap().into();
    assert_eq!(comment.kind, TokenKind::Comment("// note".into()));
    assert!("note".parse::<CommentLexeme>().is_err());
    let string: Token = "a\\n\\q".parse::<StringLexeme>().unwrap().into();
    assert_eq!(string.kind, TokenKind::String("a\n\\q".into()));
}

#[test]
fn rejects_invalid_lexemes() {
    assert!("1abc".parse::<IdentLexeme>().is_err());
    assert!("".parse::<IdentLexeme>().is_err());
    assert!("12a".parse::<NumberLexeme>().is_err());
    assert!("ab".parse::<SymbolLexeme>().is_err());
}

#[test]
fn parses_tags_and_languages() {
    let tag: MessageTag = "NATIVE_LINK_ASSIGN".parse::<TagLexeme>().unwrap().into();
    assert_eq!(tag, MessageTag::LinkAssign);
    assert_eq!(TagLexeme::from(tag).to_string(), "NATIVE_LINK_ASSIGN");
    assert!("NATIVE_NOPE".parse::<TagLexeme>().is_err());
    let language: Language = "cpp".parse::<LanguageLexeme>().unwrap().into();
    assert_eq!(language, Language::Cpp);
    assert!("rust".parse::<LanguageLexeme>().is_err());
}

#[test]
fn parses_the_event_tags_both_ways() {
    for (text, tag) in [
        ("NATIVE_HANDLER_BEGIN", MessageTag::HandlerBegin),
        ("NATIVE_HANDLER_END", MessageTag::HandlerEnd),
        ("NATIVE_EVENT_ON", MessageTag::EventOn),
        ("NATIVE_EMIT", MessageTag::Emit),
        ("NATIVE_EVENT_OFF", MessageTag::EventOff),
        ("NATIVE_EVENT_DESTROY", MessageTag::EventDestroy),
    ] {
        let parsed: MessageTag = text.parse::<TagLexeme>().unwrap().into();
        assert_eq!(parsed, tag);
        assert_eq!(TagLexeme::from(tag).to_string(), text);
    }
}

#[test]
fn parses_operators_and_paths() {
    let operator: ComparisonOperator = ">=".parse::<ComparisonLexeme>().unwrap().into();
    assert_eq!(operator, ComparisonOperator::GreaterEqual);
    assert!("=>".parse::<ComparisonLexeme>().is_err());
    let path = PathLexeme::from(Path::new("src/main/a.into"));
    assert_eq!(path.to_string(), "src/main/a.into");
}
