use testing::eval;

const STD: &str = "import \"intolang://builtin/std\";\n";

fn message_of(body: &str) -> String {
    eval(&format!("{STD}{body}")).unwrap().to_string()
}

fn error_of(body: &str) -> String {
    eval(&format!("{STD}{body}")).unwrap_err().to_string()
}

#[test]
fn registering_and_emitting_lowers_to_handler_records() {
    let message = message_of(
        "pub event example(value: Int32) {}\nexample.on((value) => { println(value); });\nexample.emit(5);\n",
    );
    assert!(
        message.contains("NATIVE_HANDLER_BEGIN\t__handler_example_"),
        "{message}"
    );
    assert!(message.contains("\tvalue:Int32"), "{message}");
    assert!(
        message.contains("NATIVE_EVENT_ON\texample\t\t__handler_example_"),
        "{message}"
    );
    assert!(message.contains("NATIVE_EMIT\texample\ti:5"), "{message}");
}

#[test]
fn the_lambda_may_name_its_parameters_freely() {
    let message = message_of(
        "pub event example(value: Int32) {}\nexample.on((renamed) => { println(renamed); });\n",
    );
    assert!(
        message.contains("NATIVE_PRINT\trenamed\tInt32"),
        "{message}"
    );
}

#[test]
fn handlers_capture_the_values_they_mention() {
    let message = message_of(
        "pub event example(value: Int32) {}\nnow into total = 5;\nnow into other = 6;\nexample.on((value) => { println(total); });\n",
    );
    assert!(message.contains("\ttotal\tvalue:Int32"), "{message}");
    assert!(!message.contains("\ttotal,other\t"), "{message}");
}

#[test]
fn the_body_of_an_event_is_its_default_handler() {
    let message = message_of("pub event example(value: Int32) {\n    println(\"default\");\n}\n");
    assert!(
        message.contains("NATIVE_HANDLER_BEGIN\t__default_example"),
        "{message}"
    );
    assert!(
        message.contains("NATIVE_EVENT_ON\texample\t\t__default_example"),
        "{message}"
    );
}

#[test]
fn on_returns_a_listener_that_unregisters_once() {
    let message = message_of(
        "pub event example(value: Int32) {}\nnow into listener = example.on((value) => { println(value); });\nlistener.destroy();\n",
    );
    assert!(
        message.contains("NATIVE_EVENT_ON\texample\tlistener\t"),
        "{message}"
    );
    assert!(
        message.contains("NATIVE_EVENT_OFF\texample\tlistener"),
        "{message}"
    );
    let error = error_of(
        "pub event example(value: Int32) {}\nnow into listener = example.on((value) => { println(value); });\nlistener.destroy();\nlistener.destroy();\n",
    );
    assert!(error.contains("`listener` was destroyed"), "{error}");
}

#[test]
fn a_listener_is_unregistered_when_its_scope_ends() {
    let message = message_of(
        "pub event example(value: Int32) {}\nnow into n = 1;\nif (n == 1) {\n    now into listener = example.on((value) => { println(value); });\n}\n",
    );
    assert!(
        message.contains("NATIVE_EVENT_OFF\texample\tlistener"),
        "{message}"
    );
}

#[test]
fn destroying_an_event_blocks_every_later_use() {
    let declared = "pub event example(value: Int32) {}\nexample.destroy();\n";
    let message = message_of(declared);
    assert!(
        message.contains("NATIVE_EVENT_DESTROY\texample"),
        "{message}"
    );
    let error = error_of(&format!("{declared}example.emit(1);\n"));
    assert!(error.contains("`example` was destroyed"), "{error}");
    let error = error_of(&format!(
        "{declared}example.on((value) => {{ println(value); }});\n"
    ));
    assert!(error.contains("`example` was destroyed"), "{error}");
    let error = error_of(&format!("{declared}example.destroy();\n"));
    assert!(error.contains("`example` was destroyed"), "{error}");
}

#[test]
fn an_event_assigned_to_a_binding_is_the_same_event() {
    let message = message_of(
        "pub event example(value: Int32) {}\nnow into e = example;\ne.on((value) => { println(value); });\ne.emit(2);\n",
    );
    assert!(
        message.contains("NATIVE_EVENT_ON\texample\t\t"),
        "{message}"
    );
    assert!(message.contains("NATIVE_EMIT\texample\ti:2"), "{message}");
    let error = error_of(
        "pub event example(value: Int32) {}\nnow into e = example;\ne.destroy();\nexample.emit(2);\n",
    );
    assert!(error.contains("`example` was destroyed"), "{error}");
    let error = error_of(
        "pub event example(value: Int32) {}\nnow into e = example;\ne.destroy();\ne.emit(2);\n",
    );
    assert!(error.contains("`e` was destroyed"), "{error}");
}

#[test]
fn emit_checks_the_arguments_against_the_declaration() {
    let error = error_of("pub event example(value: Int32) {}\nexample.emit();\n");
    assert!(error.contains("takes 1 argument"), "{error}");
    let error = error_of("pub event example(value: Int32) {}\nexample.emit(\"text\");\n");
    assert!(error.contains("expects `Int32`"), "{error}");
}

#[test]
fn the_handler_must_match_the_declared_parameters() {
    let error =
        error_of("pub event example(value: Int32) {}\nexample.on((a, b) => { println(a); });\n");
    assert!(error.contains("takes 1 parameter"), "{error}");
    let error = error_of("pub event example(value: Int32) {}\nexample.on(5);\n");
    assert!(error.contains("expects one lambda"), "{error}");
}

#[test]
fn locals_of_a_handler_are_destroyed_when_it_ends() {
    let message = eval(&format!(
        "{STD}pub Resource() {{\n    pub into: Int32 id = 1;\n    pub override fun destroy(): Void {{ println(\"released\"); }}\n}}\npub event example(value: Int32) {{}}\nexample.on((value) => {{ now into scratch = Resource(); }});\n"
    ))
    .unwrap();
    let text = message.to_string();
    let handler = text
        .split("NATIVE_HANDLER_BEGIN")
        .nth(1)
        .and_then(|rest| rest.split("NATIVE_HANDLER_END").next())
        .unwrap_or_default();
    assert!(handler.contains("released"), "{text}");
}
