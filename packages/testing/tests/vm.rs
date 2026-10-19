use intovm::TraceEntry;
use testing::eval_vm;

const STD: &str = "import \"intolang://builtin/std\";\n";

const COUNTER: &str = "pub Counter() {\n    pub into: Int32 value = 7;\n}\n";

fn run(body: &str) -> testing::VmOutcome {
    eval_vm(&format!("{STD}{COUNTER}init {{\n{body}}}\n")).unwrap()
}

fn destroyed(name: &str) -> TraceEntry {
    TraceEntry::Destroyed(name.to_owned())
}

#[test]
fn compile_time_text_and_instance_fields_are_printed() {
    let outcome =
        run("    println(\"start\");\n    now into a = Counter();\n    println(a.value);\n");
    assert_eq!(outcome.stdout, "start\n7\n");
}

#[test]
fn a_scope_end_destroys_in_reverse_declaration_order() {
    let outcome = run("    now into a = Counter();\n    now into b = Counter();\n");
    assert_eq!(outcome.trace, vec![destroyed("b"), destroyed("a")]);
}

#[test]
fn an_explicit_destroy_is_not_repeated_at_the_scope_end() {
    let outcome =
        run("    now into a = Counter();\n    now into b = Counter();\n    a.destroy();\n");
    assert_eq!(outcome.trace, vec![destroyed("a"), destroyed("b")]);
}

#[test]
fn a_moved_instance_is_destroyed_once_by_its_new_owner() {
    let outcome = run("    now into a = Counter();\n    now into b = a.move();\n");
    assert_eq!(outcome.trace, vec![destroyed("b")]);
}

#[test]
fn a_move_in_a_nested_scope_is_decided_by_the_drop_flag() {
    let outcome = run(
        "    now into a = Counter();\n    now into holder = Counter();\n    into flag: Int32 = 1;\n    if (flag == 1) {\n        now into taken = a.move();\n    }\n",
    );
    assert_eq!(outcome.trace, vec![destroyed("taken"), destroyed("holder")]);
}

#[test]
fn a_borrow_is_given_back_without_destroying_the_instance() {
    let outcome = run("    now into a = Counter();\n    now into view = a.borrow();\n");
    assert_eq!(outcome.trace, vec![destroyed("view"), destroyed("a")]);
}

#[test]
fn lowering_errors_are_reported_before_anything_runs() {
    let error = eval_vm(&format!(
        "{STD}{COUNTER}init {{\n    now into a = Counter();\n    a.destroy();\n    println(a.value);\n}}\n"
    ))
    .unwrap_err();
    assert!(error.to_string().contains("`a` was destroyed"), "{error}");
}
