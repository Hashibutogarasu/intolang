use testing::eval;

const STD: &str = "import \"intolang://builtin/std\";\n";

const COUNTER: &str = "pub Counter() {\n    pub into: Int32 value = 7;\n    pub fun peek(): Void { println(\"peek\"); }\n    #[consumes]\n    pub fun finish(): Void { println(\"finished\"); }\n}\n";

fn program(body: &str) -> String {
    format!("{STD}{COUNTER}{body}")
}

fn text_of(body: &str) -> String {
    eval(&program(body)).unwrap().text()
}

fn error_of(body: &str) -> String {
    eval(&program(body)).unwrap_err().to_string()
}

#[test]
fn a_reference_parameter_borrows_the_argument_for_the_call() {
    let text = text_of(
        "pub fun look(target: Counter&) { target.peek(); }\nnow into a = Counter();\nlook(a);\na.peek();\n",
    );
    assert_eq!(text, "peek\npeek\n");
}

#[test]
fn a_reference_cannot_call_a_consuming_method() {
    let error = error_of(
        "pub fun release(target: Counter&) { target.finish(); }\nnow into a = Counter();\nrelease(a);\n",
    );
    assert!(error.contains("requires ownership"), "{error}");
}

#[test]
fn a_reference_cannot_move_the_instance() {
    let error = error_of(
        "pub fun release(target: Counter&) { target.move(); }\nnow into a = Counter();\nrelease(a);\n",
    );
    assert!(error.contains("requires ownership"), "{error}");
}

#[test]
fn the_owner_is_usable_again_after_the_call() {
    let text = text_of(
        "pub fun look(target: Counter&) { target.peek(); }\nnow into a = Counter();\nlook(a);\na.finish();\n",
    );
    assert_eq!(text, "peek\nfinished\n");
}

#[test]
fn a_consuming_method_leaves_the_owner_moved() {
    let error = error_of("now into a = Counter();\na.finish();\na.peek();\n");
    assert!(error.contains("`a` was moved"), "{error}");
}

#[test]
fn a_detaching_method_leaves_the_owner_detached() {
    let source = format!(
        "{STD}pub Job() {{\n    pub into: Int32 id = 1;\n    pub fun wait(): Void {{ println(\"waited\"); }}\n    #[consumes(\"detached\")]\n    pub fun detach(): Void {{ println(\"detached\"); }}\n}}\nnow into job = Job();\njob.detach();\njob.wait();\n"
    );
    let error = eval(&source).unwrap_err().to_string();
    assert!(error.contains("`job` was detached"), "{error}");
}

#[test]
fn borrow_produces_a_reference_that_cannot_consume() {
    let error = error_of("now into a = Counter();\nnow into view = a.borrow();\nview.finish();\n");
    assert!(error.contains("requires ownership"), "{error}");
    let text = text_of("now into a = Counter();\nnow into view = a.borrow();\nview.peek();\n");
    assert_eq!(text, "peek\n");
}

#[test]
fn a_reference_cannot_be_bound_to_an_owning_annotation() {
    let error = error_of("now into a = Counter();\nnow into view: Counter = a.borrow();\n");
    assert!(error.contains("Counter"), "{error}");
}

#[test]
fn a_reference_cannot_be_passed_where_a_value_is_expected() {
    let error = error_of(
        "pub fun take(target: Counter) { target.peek(); }\nnow into a = Counter();\nnow into view = a.borrow();\ntake(view);\n",
    );
    assert!(error.contains("is a reference"), "{error}");
}

#[test]
fn a_reference_annotation_borrows_the_value() {
    let text = text_of("now into a = Counter();\nnow into view: Counter& = a;\nview.peek();\n");
    assert_eq!(text, "peek\n");
    let error = error_of("now into a = Counter();\nnow into view: Counter& = a;\na.finish();\n");
    assert!(
        error.contains("`a` was moved") || error.contains("is borrowed"),
        "{error}"
    );
}

#[test]
fn a_function_may_return_a_reference_to_its_argument() {
    let text = text_of(
        "pub fun pick(target: Counter&): Counter& { return target; }\nnow into a = Counter();\nnow into found = pick(a);\nfound.peek();\n",
    );
    assert_eq!(text, "peek\n");
}

#[test]
fn the_result_of_a_reference_function_owns_nothing() {
    let error = error_of(
        "pub fun pick(target: Counter&): Counter& { return target; }\nnow into a = Counter();\nnow into found = pick(a);\nfound.finish();\n",
    );
    assert!(error.contains("requires ownership"), "{error}");
    let error = error_of(
        "pub fun pick(target: Counter&): Counter& { return target; }\nnow into a = Counter();\nnow into found: Counter = pick(a);\n",
    );
    assert!(error.contains("Counter"), "{error}");
    let error = error_of(
        "pub fun pick(target: Counter&): Counter& { return target; }\nnow into a = Counter();\nnow into found = pick(a);\na.finish();\n",
    );
    assert!(error.contains("is borrowed"), "{error}");
}

#[test]
fn a_function_cannot_return_a_reference_to_a_local() {
    let error = error_of(
        "pub fun local(): Counter& { now into inner = Counter(); return inner; }\nnow into found = local();\n",
    );
    assert!(error.contains("returns a reference to a local"), "{error}");
}

#[test]
fn moving_an_argument_hands_the_instance_to_the_function() {
    let error = error_of(
        "pub fun consume(owned: Counter) { owned.finish(); }\nnow into a = Counter();\nconsume(a.move());\na.peek();\n",
    );
    assert!(error.contains("`a` was moved"), "{error}");
    let text = text_of(
        "pub fun consume(owned: Counter) { owned.finish(); }\nnow into a = Counter();\nconsume(a.move());\n",
    );
    assert_eq!(text, "finished\n");
}

#[test]
fn copying_an_argument_keeps_the_original() {
    let text = text_of(
        "pub fun consume(owned: Counter) { owned.finish(); }\nnow into a = Counter();\nconsume(a.copy());\na.peek();\n",
    );
    assert_eq!(text, "finished\npeek\n");
}

#[test]
fn a_borrowed_argument_is_given_back_after_the_call() {
    let text = text_of(
        "pub fun look(target: Counter&) { target.peek(); }\nnow into a = Counter();\nlook(a.borrow());\na.finish();\n",
    );
    assert_eq!(text, "peek\nfinished\n");
}
