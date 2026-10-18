use testing::eval;

const STD: &str = "import \"intolang://builtin/std\";\n";

const COUNTER: &str = "pub Counter() {\n    pub into: Int32 value = 7;\n}\n";

fn with(body: &str) -> String {
    format!("{STD}{COUNTER}init {{\n{body}}}\n")
}

fn error_of(body: &str) -> String {
    eval(&with(body)).unwrap_err().to_string()
}

#[test]
fn destroy_blocks_later_use() {
    let error = error_of("    now into a = Counter();\n    a.destroy();\n    println(a.value);\n");
    assert!(error.contains("`a` was destroyed"), "{error}");
}

#[test]
fn destroy_blocks_calling_methods_again() {
    let error = error_of("    now into a = Counter();\n    a.destroy();\n    a.destroy();\n");
    assert!(error.contains("`a` was destroyed"), "{error}");
}

#[test]
fn using_before_destroy_is_fine() {
    let output = eval(&with(
        "    now into a = Counter();\n    println(a.value);\n    a.destroy();\n",
    ))
    .unwrap();
    assert!(
        output.contains("NATIVE_FIELD_PRINT\ta\tCounter"),
        "{output}"
    );
}

#[test]
fn move_transfers_the_instance_and_invalidates_the_source() {
    let output = eval(&with(
        "    now into a = Counter();\n    now into b = a.move();\n    println(b.value);\n",
    ))
    .unwrap();
    assert!(output.contains("NATIVE_ALIAS\tb\ta"), "{output}");
    let error = error_of(
        "    now into a = Counter();\n    now into b = a.move();\n    println(a.value);\n",
    );
    assert!(error.contains("`a` was moved"), "{error}");
}

#[test]
fn moved_bindings_cannot_be_copied_or_moved_again() {
    let error = error_of(
        "    now into a = Counter();\n    now into b = a.move();\n    now into c = a.copy();\n",
    );
    assert!(error.contains("`a` was moved"), "{error}");
    let error =
        error_of("    now into a = Counter();\n    now into b = a.move();\n    now into c = a;\n");
    assert!(error.contains("`a` was moved"), "{error}");
}

#[test]
fn copy_creates_an_independent_instance() {
    let output = eval(&with(
        "    now into a = Counter();\n    now into b = a.copy();\n    println(a.value);\n    println(b.value);\n    b.destroy();\n    println(a.value);\n",
    ))
    .unwrap();
    assert!(output.contains("NATIVE_COPY\tb\ta\tCounter"), "{output}");
}

#[test]
fn borrow_keeps_the_owner_usable_but_locked() {
    let output = eval(&with(
        "    now into a = Counter();\n    now into view = a.borrow();\n    println(a.value);\n    println(view.value);\n",
    ))
    .unwrap();
    assert!(output.contains("NATIVE_ALIAS\tview\ta"), "{output}");
    let error = error_of(
        "    now into a = Counter();\n    now into view = a.borrow();\n    a.destroy();\n",
    );
    assert!(error.contains("`a` is borrowed"), "{error}");
    let error = error_of(
        "    now into a = Counter();\n    now into view = a.borrow();\n    now into b = a.move();\n",
    );
    assert!(error.contains("`a` is borrowed"), "{error}");
}

#[test]
fn returning_a_borrow_releases_the_owner() {
    let output = eval(&with(
        "    now into a = Counter();\n    now into view = a.borrow();\n    view.destroy();\n    a.destroy();\n",
    ))
    .unwrap();
    assert!(output.contains("NATIVE_ALIAS\tview\ta"), "{output}");
}

#[test]
fn a_returned_borrow_cannot_be_used_again() {
    let error = error_of(
        "    now into a = Counter();\n    now into view = a.borrow();\n    view.destroy();\n    println(view.value);\n",
    );
    assert!(error.contains("`view` was destroyed"), "{error}");
}

#[test]
fn several_borrows_can_coexist() {
    let error = error_of(
        "    now into a = Counter();\n    now into one = a.borrow();\n    now into two = a.borrow();\n    one.destroy();\n    a.destroy();\n",
    );
    assert!(error.contains("`a` is borrowed"), "{error}");
    let output = eval(&with(
        "    now into a = Counter();\n    now into one = a.borrow();\n    now into two = a.borrow();\n    one.destroy();\n    two.destroy();\n    a.destroy();\n",
    ))
    .unwrap();
    assert!(output.contains("NATIVE_ALIAS\ttwo\ta"), "{output}");
}

#[test]
fn an_overriding_destroy_still_marks_the_instance_destroyed() {
    let source = format!(
        "{STD}pub Resource() {{\n    pub into: Int32 id = 1;\n    pub override fun destroy(): Void {{ println(\"released\"); }}\n}}\ninit {{\n    now into r = Resource();\n    r.destroy();\n    r.destroy();\n}}\n"
    );
    let error = eval(&source).unwrap_err().to_string();
    assert!(error.contains("`r` was destroyed"), "{error}");
    let source = format!(
        "{STD}pub Resource() {{\n    pub into: Int32 id = 1;\n    pub override fun destroy(): Void {{ println(\"released\"); }}\n}}\ninit {{\n    now into r = Resource();\n    r.destroy();\n}}\n"
    );
    assert_eq!(eval(&source).unwrap().text(), "released\n");
}
