use ast::{Declaration, Expr, Program, StmtKind};
use types::Literal;

fn parse(source: &str) -> Expr {
    source.parse().unwrap()
}

#[test]
fn an_array_literal_lists_its_items() {
    assert_eq!(
        parse("[1, 2, 3]"),
        Expr::Array(vec![Expr::int(1), Expr::int(2), Expr::int(3)])
    );
    assert_eq!(parse("[]"), Expr::Array(Vec::new()));
}

#[test]
fn a_trailing_comma_is_accepted() {
    assert_eq!(
        parse(r#"["a", "b",]"#),
        Expr::Array(vec![
            Expr::Literal(Literal::String("a".into())),
            Expr::Literal(Literal::String("b".into())),
        ])
    );
}

#[test]
fn arrays_nest_and_hold_expressions() {
    assert_eq!(parse("[[1], [a + 1]]").to_string(), "[[1], [a + 1]]");
}

#[test]
fn an_unclosed_array_is_an_error() {
    assert!("[1, 2".parse::<Expr>().is_err());
    assert!("[1 2]".parse::<Expr>().is_err());
}

#[test]
fn an_array_type_is_written_with_brackets() {
    let program: Program = r#"init {
    into xs: [Int32] = [1];
    into grid: [[String]] = [];
}
"#
    .parse()
    .unwrap();
    let body = program
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
    let annotations: Vec<String> = body
        .iter()
        .filter_map(|stmt| match &stmt.kind {
            StmtKind::Declaration(Declaration { annotation, .. }) => {
                annotation.as_ref().map(ToString::to_string)
            }
            _ => None,
        })
        .collect();
    assert_eq!(annotations, vec!["[Int32]", "[[String]]"]);
}
