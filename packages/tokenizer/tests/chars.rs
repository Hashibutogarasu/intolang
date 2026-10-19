use tokenizer::Tokens;
use types::{Token, TokenKind};

fn kinds(source: &str) -> Vec<TokenKind> {
    let tokens: Vec<Token> = source.parse::<Tokens>().unwrap().into();
    tokens.into_iter().map(|token| token.kind).collect()
}

#[test]
fn a_character_literal_is_one_token() {
    assert_eq!(
        kinds("into c = 'A';"),
        vec![
            TokenKind::Identifier("into".into()),
            TokenKind::Identifier("c".into()),
            TokenKind::Symbol('='),
            TokenKind::Char('A'),
            TokenKind::Symbol(';'),
        ]
    );
}

#[test]
fn escapes_are_decoded() {
    assert_eq!(
        kinds(r"'\n' '\'' '\\' '\t'"),
        vec![
            TokenKind::Char('\n'),
            TokenKind::Char('\''),
            TokenKind::Char('\\'),
            TokenKind::Char('\t'),
        ]
    );
}

#[test]
fn a_character_literal_may_hold_a_non_ascii_character() {
    assert_eq!(kinds("'あ'"), vec![TokenKind::Char('あ')]);
}

#[test]
fn the_span_covers_the_quotes() {
    let tokens = "x 'A'".parse::<Tokens>().unwrap();
    assert_eq!((tokens[1].span.start, tokens[1].span.end), (2, 5));
}

#[test]
fn an_unterminated_or_long_literal_is_rejected() {
    assert!("'A".parse::<Tokens>().is_err());
    assert!("'AB'".parse::<Tokens>().is_err());
    assert!("''".parse::<Tokens>().is_err());
}
