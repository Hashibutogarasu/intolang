use ast::Program;

fn expected(source: &str) -> Vec<String> {
    let error = source.parse::<Program>().unwrap_err();
    error
        .resolved_context()
        .syntax
        .expect("a syntax error has a syntax context")
        .expected
}

#[test]
fn a_line_break_does_not_end_a_statement() {
    let source = r#"into x = 1
into y = 2;
"#;
    assert_eq!(expected(source), vec!["`;`".to_string()]);
}

#[test]
fn a_closing_brace_does_not_end_a_statement() {
    assert_eq!(expected("init { f() }"), vec!["`;`".to_string()]);
}

#[test]
fn the_end_of_the_input_does_not_end_a_statement() {
    assert_eq!(expected("break"), vec!["`;`".to_string()]);
}
