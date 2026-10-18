use tokenizer::Tokens;
use types::{Token, TokenKind};

fn kinds(source: &str) -> Vec<TokenKind> {
    let tokens: Vec<Token> = source.parse::<Tokens>().unwrap().into();
    tokens.into_iter().map(|token| token.kind).collect()
}

#[test]
fn skips_whitespace_and_comments() {
    assert_eq!(
        kinds("into x = 1; // trailing\n  x"),
        vec![
            TokenKind::Identifier("into".into()),
            TokenKind::Identifier("x".into()),
            TokenKind::Symbol('='),
            TokenKind::Number("1".into()),
            TokenKind::Symbol(';'),
            TokenKind::Identifier("x".into()),
        ]
    );
}

#[test]
fn decodes_string_escapes() {
    assert_eq!(
        kinds("\"a\\n\\t\\\"\\\\\\q\""),
        vec![TokenKind::String("a\n\t\"\\\\q".into())]
    );
}

#[test]
fn tracks_lines_including_multiline_strings() {
    let tokens = "a\n\"x\ny\"\nb".parse::<Tokens>().unwrap();
    let lines: Vec<usize> = tokens.iter().map(|token| token.line()).collect();
    assert_eq!(lines, vec![1, 2, 4]);
    assert_eq!(tokens[1].span.end_line, 3);
}

#[test]
fn records_character_spans() {
    let tokens = "ab = \"x\";".parse::<Tokens>().unwrap();
    let spans: Vec<(usize, usize)> = tokens
        .iter()
        .map(|token| (token.span.start, token.span.end))
        .collect();
    assert_eq!(spans, vec![(0, 2), (3, 4), (5, 8), (8, 9)]);
}

#[test]
fn keeps_comments_only_on_request() {
    let source = "a; // note\nb;";
    let plain = source.parse::<Tokens>().unwrap();
    assert_eq!(plain.len(), 4);
    let with_comments = Tokens::with_comments(source).unwrap();
    assert_eq!(with_comments.len(), 5);
    assert_eq!(with_comments[2].kind, TokenKind::Comment("// note".into()));
}

#[test]
fn rejects_unterminated_strings() {
    assert!("\"open".parse::<Tokens>().is_err());
    assert!("\"open\\\"".parse::<Tokens>().is_err());
    assert!("\"open\\".parse::<Tokens>().is_err());
}

#[test]
fn keeps_unknown_characters_as_symbols() {
    assert_eq!(
        kinds("a->b*"),
        vec![
            TokenKind::Identifier("a".into()),
            TokenKind::Symbol('-'),
            TokenKind::Symbol('>'),
            TokenKind::Identifier("b".into()),
            TokenKind::Symbol('*'),
        ]
    );
}
