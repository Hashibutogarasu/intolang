use parser::{CompoundLexeme, KeywordLexeme, PunctLexeme};
use types::{CompoundOperator, Keyword, Punct};

#[test]
fn parses_keywords_both_ways() {
    let keyword: Keyword = "unload".parse::<KeywordLexeme>().unwrap().into();
    assert_eq!(keyword, Keyword::Unload);
    assert_eq!(KeywordLexeme::from(Keyword::Else).to_string(), "else");
    assert!("intoVar".parse::<KeywordLexeme>().is_err());
}

#[test]
fn converts_punctuation_through_char() {
    let punct: Punct = PunctLexeme::try_from('{').unwrap().into();
    assert_eq!(punct, Punct::LeftBrace);
    assert_eq!(char::from(PunctLexeme::from(Punct::Hash)), '#');
    assert!(PunctLexeme::try_from('+').is_err());
    assert!("ab".parse::<PunctLexeme>().is_err());
}

#[test]
fn builds_compound_operators_from_symbol_pairs() {
    let operator: CompoundOperator = CompoundLexeme::try_from(('!', '='))
        .unwrap()
        .into();
    assert_eq!(operator, CompoundOperator::NotEqual);
    assert_eq!(CompoundLexeme::from(CompoundOperator::And).to_string(), "&&");
    assert!(CompoundLexeme::try_from(('=', '>')).is_err());
}
