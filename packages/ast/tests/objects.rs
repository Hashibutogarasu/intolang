use ast::{Expr, Program, StmtKind};
use types::Literal;

fn parse(source: &str) -> Expr {
    source.parse().unwrap()
}

fn text(value: &str) -> Expr {
    Expr::Literal(Literal::String(value.to_owned()))
}

#[test]
fn an_object_literal_lists_its_entries() {
    assert_eq!(
        parse(r#"{ "foo": "bar", "number": 0 }"#),
        Expr::Object(vec![
            ("foo".to_owned(), text("bar")),
            ("number".to_owned(), Expr::int(0)),
        ])
    );
    assert_eq!(parse("{}"), Expr::Object(Vec::new()));
}

#[test]
fn a_trailing_comma_and_nesting_are_accepted() {
    assert_eq!(
        parse(r#"{ "inner": { "a": [1, 2], }, }"#),
        Expr::Object(vec![(
            "inner".to_owned(),
            Expr::Object(vec![(
                "a".to_owned(),
                Expr::Array(vec![Expr::int(1), Expr::int(2)])
            )])
        )])
    );
}

#[test]
fn the_values_are_expressions() {
    assert_eq!(
        parse(r#"{ "sum": a + 1 }"#).to_string(),
        r#"{ "sum": a + 1 }"#
    );
    assert_eq!(
        parse(r#"{ "a": 1, "b": 2 }"#).to_string(),
        r#"{ "a": 1, "b": 2 }"#
    );
    assert_eq!(parse("{}").to_string(), "{}");
}

#[test]
fn a_key_must_be_text_and_entries_need_commas() {
    assert!("{ foo: 1 }".parse::<Expr>().is_err());
    assert!(r#"{ "a": 1 "b": 2 }"#.parse::<Expr>().is_err());
    assert!(r#"{ "a" 1 }"#.parse::<Expr>().is_err());
    assert!(r#"{ "a": 1"#.parse::<Expr>().is_err());
}

#[test]
fn a_block_after_a_condition_is_not_an_object() {
    let program: Program = r#"init {
    if (a == 1) {
        b = 2;
    }
    while (a < 3) {
        a = a + 1;
    }
}
"#
    .parse()
    .unwrap();
    let init = program
        .items
        .iter()
        .find_map(|item| match item {
            ast::Item::Stmt(stmt) => match &stmt.kind {
                StmtKind::Init(body) => Some(body),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    assert_eq!(init.len(), 2);
}

#[test]
fn a_declaration_can_hold_an_object() {
    let program: Program = r#"now into json: Json = { "foo": "bar" };
"#
    .parse()
    .unwrap();
    let declared = program.items.iter().any(|item| match item {
        ast::Item::Stmt(stmt) => matches!(
            &stmt.kind,
            StmtKind::Declaration(declaration) if matches!(declaration.value, Some(Expr::Object(_)))
        ),
        _ => false,
    });
    assert!(declared);
}
