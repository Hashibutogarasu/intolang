use testing::eval;

const STD: &str = "import \"intolang://builtin/std\";\n";

const CLASSES: &str = "pub First() {\n    pub into: Int32 id = 1;\n    pub override fun destroy(): Void { println(\"first\"); }\n}\npub Second() {\n    pub into: Int32 id = 2;\n    pub override fun destroy(): Void { println(\"second\"); }\n}\n";

fn program(body: &str) -> String {
    format!("{STD}{CLASSES}{body}")
}

fn text_of(body: &str) -> String {
    eval(&program(body)).unwrap().text()
}

fn error_of(body: &str) -> String {
    eval(&program(body)).unwrap_err().to_string()
}

#[test]
fn the_program_end_destroys_in_reverse_order() {
    let text = text_of("now into a = First();\nnow into b = Second();\n");
    assert_eq!(text, "second\nfirst\n");
}

#[test]
fn init_destroys_its_bindings_when_it_ends() {
    let text = text_of("init {\n    now into a = First();\n    now into b = Second();\n}\n");
    assert_eq!(text, "second\nfirst\n");
}

#[test]
fn an_if_block_destroys_its_bindings_at_its_end() {
    let text = text_of(
        "now into n = 1;\nif (n == 1) {\n    now into a = First();\n}\nnow into b = Second();\n",
    );
    assert_eq!(text, "first\nsecond\n");
}

#[test]
fn a_binding_cannot_be_used_after_its_scope_ended() {
    let error =
        error_of("now into n = 1;\nif (n == 1) {\n    now into a = First();\n}\nprintln(a.id);\n");
    assert!(error.contains("`a` was destroyed"), "{error}");
}

#[test]
fn an_explicit_destroy_is_not_repeated_at_the_scope_end() {
    let text = text_of("now into a = First();\na.destroy();\n");
    assert_eq!(text, "first\n");
}

#[test]
fn a_moved_binding_is_destroyed_once_by_its_new_owner() {
    let text = text_of("now into a = First();\nnow into b = a.move();\n");
    assert_eq!(text, "first\n");
}

#[test]
fn a_copy_is_destroyed_separately_from_its_source() {
    let text = text_of("now into a = First();\nnow into b = a.copy();\n");
    assert_eq!(text, "first\nfirst\n");
}

#[test]
fn a_borrow_is_given_back_without_destroying_the_instance() {
    let text = text_of(
        "now into a = First();\nnow into n = 1;\nif (n == 1) {\n    now into view = a.borrow();\n}\nnow into b = a.move();\n",
    );
    assert_eq!(text, "first\n");
}

#[test]
fn an_alias_does_not_destroy_the_instance_twice() {
    let text = text_of("now into a = First();\nnow into b = a;\n");
    assert_eq!(text, "first\n");
}

#[test]
fn break_destroys_what_the_loop_body_owns() {
    let output = eval(&program(
        "now into n = 0;\nwhile (n < 2) {\n    now into a = First();\n    break;\n}\n",
    ))
    .unwrap();
    assert_eq!(output.text(), "first\nfirst\n", "{output}");
}

#[test]
fn a_move_in_a_nested_scope_is_decided_by_a_drop_flag() {
    let output = eval(&program(
        "now into a = First();\nnow into n = 1;\nif (n == 1) {\n    now into b = a.move();\n}\n",
    ))
    .unwrap();
    assert!(output.contains("__drop_a"), "{output}");
}
