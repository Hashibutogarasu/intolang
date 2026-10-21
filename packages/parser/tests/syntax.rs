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
fn lists_every_keyword_once() {
    let spellings: Vec<String> = KeywordLexeme::all()
        .map(|lexeme| lexeme.to_string())
        .collect();
    for spelling in &spellings {
        let lexeme: KeywordLexeme = spelling.parse().unwrap();
        assert_eq!(&lexeme.to_string(), spelling);
    }
    let mut unique = spellings.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), spellings.len());
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
    let operator: CompoundOperator = CompoundLexeme::try_from(('!', '=')).unwrap().into();
    assert_eq!(operator, CompoundOperator::NotEqual);
    assert_eq!(
        CompoundLexeme::from(CompoundOperator::And).to_string(),
        "&&"
    );
    let arrow: CompoundOperator = CompoundLexeme::try_from(('=', '>')).unwrap().into();
    assert_eq!(arrow, CompoundOperator::Arrow);
    assert_eq!(
        CompoundLexeme::from(CompoundOperator::Arrow).to_string(),
        "=>"
    );
    assert!(CompoundLexeme::try_from(('=', '<')).is_err());
}

#[test]
fn parses_the_object_model_keywords() {
    for (text, keyword) in [
        ("event", Keyword::Event),
        ("thread", Keyword::Thread),
        ("await", Keyword::Await),
        ("abstract", Keyword::Abstract),
        ("default", Keyword::Default),
        ("override", Keyword::Override),
        ("as", Keyword::As),
    ] {
        let parsed: Keyword = text.parse::<KeywordLexeme>().unwrap().into();
        assert_eq!(parsed, keyword);
        assert_eq!(KeywordLexeme::from(keyword).to_string(), text);
    }
}
