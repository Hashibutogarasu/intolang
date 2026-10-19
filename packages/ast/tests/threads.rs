use ast::{Expr, Program, StmtKind};

fn expression_of(source: &str) -> Expr {
    let program: Program = source.parse().unwrap();
    match &program.items.iter().find_map(|item| match item {
        ast::Item::Stmt(stmt) => Some(&stmt.kind),
        _ => None,
    }) {
        Some(StmtKind::Declaration(declaration)) => declaration.value.clone().unwrap(),
        Some(StmtKind::Expr(expression)) => expression.clone(),
        other => panic!("unexpected statement {other:?}"),
    }
}

#[test]
fn parses_thread_expressions() {
    let Expr::Thread { body } = expression_of(r#"now into worker = thread { println("hi"); };"#)
    else {
        panic!("expected a thread");
    };
    assert_eq!(body.len(), 1);
}

#[test]
fn parses_await_of_a_run_call_and_of_a_task() {
    let Expr::Await(inner) = expression_of("await worker.run();") else {
        panic!("expected an await");
    };
    assert!(matches!(*inner, Expr::Call { .. }));
    let Expr::Await(inner) = expression_of("await task;") else {
        panic!("expected an await");
    };
    assert_eq!(inner.as_ident(), Some("task"));
}

#[test]
fn thread_is_only_a_keyword_before_a_brace() {
    assert!(matches!(
        expression_of("now into thread = 1;"),
        Expr::Literal(_)
    ));
}
