use ast::{Expr, Program, StmtKind};

fn program(source: &str) -> Program {
    source.parse().unwrap()
}

#[test]
fn parses_event_declarations() {
    let program = program(
        r#"pub event example(args: Args) {
}
event quiet() {}
"#,
    );
    let events: Vec<_> = program.events().collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].name, "example");
    assert!(events[0].is_public);
    assert_eq!(events[0].params.len(), 1);
    assert_eq!(events[0].params[0].name, "args");
    assert!(events[0].body.is_empty());
    assert!(!events[1].is_public);
    assert!(events[1].params.is_empty());
}

#[test]
fn parses_event_modifiers_and_type_parameters() {
    let program = program(
        r#"pub default event onCast<From, To>(from: From, to: To) {}
pub override event onCast<Char, Hex>(from: Char, to: Hex) {}
"#,
    );
    let events: Vec<_> = program.events().collect();
    assert!(events[0].is_default && !events[0].is_override);
    assert_eq!(events[0].type_params, vec!["From", "To"]);
    assert!(events[1].is_override && !events[1].is_default);
    assert_eq!(events[1].type_params, vec!["Char", "Hex"]);
}

#[test]
fn parses_a_lambda_argument() {
    let program = program(
        r#"example.on((args) => {
    println("got");
});
"#,
    );
    let stmt = program.statements().next().unwrap();
    let StmtKind::Expr(Expr::Call { callee, args }) = &stmt.kind else {
        panic!("expected a call statement, found {:?}", stmt.kind);
    };
    assert_eq!(callee.to_string(), "example.on");
    let Expr::Lambda { params, body } = &args[0] else {
        panic!("expected a lambda argument, found {:?}", args[0]);
    };
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].name, "args");
    assert_eq!(body.len(), 1);
}

#[test]
fn parses_typed_and_empty_lambda_parameters() {
    let program = program(
        r#"run(() => {});
run((a: Int32, b) => { });
"#,
    );
    let lambdas: Vec<_> = program
        .statements()
        .map(|stmt| match &stmt.kind {
            StmtKind::Expr(Expr::Call { args, .. }) => match &args[0] {
                Expr::Lambda { params, .. } => params.len(),
                other => panic!("expected a lambda, found {other:?}"),
            },
            other => panic!("expected a call, found {other:?}"),
        })
        .collect();
    assert_eq!(lambdas, vec![0, 2]);
}

#[test]
fn parenthesised_expressions_are_not_lambdas() {
    let program = program(
        r#"into value = (1 + 2);
"#,
    );
    let stmt = program.statements().next().unwrap();
    assert!(matches!(stmt.kind, StmtKind::Declaration(_)));
}

#[test]
fn lambda_parameters_shadow_inlined_names() {
    let program = program(
        r#"example.on((args) => { use(args); });
"#,
    );
    let stmt = program.statements().next().unwrap();
    let replaced = stmt.substitute(&[("args".to_owned(), Expr::int(7))]);
    let StmtKind::Expr(Expr::Call { args, .. }) = &replaced.kind else {
        panic!("expected a call statement");
    };
    let Expr::Lambda { body, .. } = &args[0] else {
        panic!("expected a lambda argument");
    };
    let StmtKind::Expr(Expr::Call { args, .. }) = &body[0].kind else {
        panic!("expected a call in the body");
    };
    assert_eq!(args[0].as_ident(), Some("args"));
}
