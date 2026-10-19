use ast::{Declaration, Expr, Item, Program, StmtKind, SyntaxError};
use types::{ArithmeticOperator, ComparisonOperator, Literal, Storage};

fn statements(source: &str) -> Vec<StmtKind> {
    source
        .parse::<Program>()
        .unwrap()
        .statements()
        .map(|stmt| stmt.kind.clone())
        .collect()
}

fn declaration(source: &str) -> Declaration {
    match statements(source).remove(0) {
        StmtKind::Declaration(declaration) => declaration,
        other => panic!("expected a declaration, found {other:?}"),
    }
}

#[test]
fn parses_declaration_forms() {
    let plain = declaration("into x: Int32 = 5;");
    assert_eq!(plain.storage, Storage::Local);
    assert_eq!(plain.name, "x");
    assert_eq!(plain.annotation.unwrap().as_str(), "Int32");
    assert_eq!(plain.value, Some(Expr::int(5)));

    let constant = declaration("const now into limit = 3;");
    assert!(constant.constant);
    assert_eq!(constant.storage, Storage::Now);

    let late = declaration(r#"late into lazy = "x";"#);
    assert_eq!(late.storage, Storage::Late);

    let pointer = declaration("now into* handle = f();");
    assert!(pointer.pointer);

    let bare = declaration("now into *p;");
    assert!(bare.pointer && bare.value.is_none());

    let generic = declaration("into items: List<Int32>* = make();");
    assert_eq!(generic.annotation.unwrap().as_str(), "List<Int32>*");
}

#[test]
fn parses_expressions() {
    let value = declaration("into x = a + 1;").value.unwrap();
    assert_eq!(
        value,
        Expr::Binary {
            left: Box::new(Expr::ident("a")),
            op: ArithmeticOperator::Add,
            right: Box::new(Expr::int(1)),
        }
    );
    assert_eq!(
        declaration("into r: Float = 1.5;").value,
        Some(Expr::Literal(Literal::Float(1.5)))
    );
    assert_eq!(declaration("into n = -7;").value, Some(Expr::int(-7)));
    assert_eq!(
        declaration(r#"into s = "a\nb";"#).value,
        Some(Expr::Literal(Literal::String("a\nb".into())))
    );
    let call = declaration(r#"into v = w.window.create(1, "t", x);"#)
        .value
        .unwrap();
    assert_eq!(call.to_string(), r#"w.window.create(1, "t", x)"#);
    let negated = declaration("into ok = !check(x);").value.unwrap();
    assert_eq!(negated.to_string(), "!check(x)");
}

#[test]
fn parses_control_flow() {
    let StmtKind::While { condition, body } =
        statements("while (i < 3) { i = i + 1; if (i == 2) { break; } else { f(); } }").remove(0)
    else {
        panic!("expected while");
    };
    assert!(matches!(
        condition,
        Expr::Compare {
            op: ComparisonOperator::Less,
            ..
        }
    ));
    assert_eq!(body.len(), 2);
    let StmtKind::If {
        condition,
        then_body,
        else_body,
    } = &body[1].kind
    else {
        panic!("expected if");
    };
    assert!(matches!(
        condition,
        Expr::Compare {
            op: ComparisonOperator::Equal,
            ..
        }
    ));
    assert_eq!(then_body.len(), 1);
    assert_eq!(else_body.as_ref().unwrap().len(), 1);
}

#[test]
fn accepts_conditions_without_parentheses_and_else_if() {
    let StmtKind::If { else_body, .. } =
        statements("if condition == false { f(); } else if other { g(); } else { h(); }").remove(0)
    else {
        panic!("expected if");
    };
    let else_body = else_body.unwrap();
    assert!(matches!(else_body[0].kind, StmtKind::If { .. }));
}

#[test]
fn distinguishes_assignment_from_comparison() {
    assert!(matches!(
        statements("x = 1;").remove(0),
        StmtKind::Assign { .. }
    ));
    assert!(matches!(
        statements("x == 1;").remove(0),
        StmtKind::Expr(Expr::Compare { .. })
    ));
}

#[test]
fn semicolons_are_optional_at_line_and_block_ends() {
    let kinds = statements(
        r#"break
print("x")
return
unload y"#,
    );
    assert_eq!(kinds.len(), 4);
    let nested = statements("init { f() }");
    assert!(matches!(nested[0], StmtKind::Init(_)));
}

#[test]
fn statements_do_not_depend_on_layout() {
    let spread = r#"into x
=
1
+
2
;
f(
a,
"b"
)
;
"#;
    let packed = r#"into x=1+2;f(a,"b");"#;
    assert_eq!(statements(spread), statements(packed));
}

#[test]
fn reports_syntax_errors_with_lines() {
    let error = r#"into a = 1;
f(;"#
        .parse::<Program>()
        .unwrap_err();
    assert_eq!(error.downcast_ref::<SyntaxError>().unwrap().line, 2);
    let error = "}".parse::<Program>().unwrap_err();
    assert!(error.to_string().contains("Unexpected closing brace"));
    assert!("init {".parse::<Program>().is_err());
}

#[test]
fn rejects_nested_function_declarations() {
    let error = r#"pub fun outer() {
    pub fun inner() {}
}
"#
    .parse::<Program>()
    .unwrap_err();
    let syntax = error.downcast_ref::<SyntaxError>().unwrap();
    assert_eq!(syntax.line, 2);
    assert_eq!(syntax.scope, "function body");
    assert!(syntax.message.contains("cannot be nested"));
    assert!(matches!(
        "pub fun outer() { init { f(); } }"
            .parse::<Program>()
            .unwrap()
            .items[0],
        Item::Function(_)
    ));
}
