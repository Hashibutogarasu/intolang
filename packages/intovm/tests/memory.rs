mod support;

use support::{record, run, run_error};
use types::MessageTag::*;

fn declare(name: &str, storage: &str) -> String {
    record(Ownership, &["declare", name, storage])
}

fn change(op: &str, fields: &[&str]) -> String {
    let mut all = vec![op];
    all.extend_from_slice(fields);
    record(Ownership, &all)
}

fn expect(name: &str, property: &str, value: bool) -> String {
    let flag = if value { "1" } else { "0" };
    [
        record(IfState, &[name, property, flag]),
        "ok\n".to_owned(),
        record(IfElse, &[]),
        record(Throw, &["6661696c"]),
        record(IfEnd, &[]),
    ]
    .concat()
}

#[test]
fn a_now_binding_is_in_memory_until_it_is_unloaded() {
    let message = [
        declare("a", "now"),
        expect("a", "isInMemory", true),
        change("unload", &["a"]),
        expect("a", "isInMemory", false),
    ]
    .concat();
    assert_eq!(run(&message), "ok\nok\n");
}

#[test]
fn a_late_binding_is_in_memory_once_it_is_read() {
    let message = [declare("a", "late"), expect("a", "isInMemory", true)].concat();
    assert_eq!(run(&message), "ok\n");
}

#[test]
fn a_local_binding_is_in_memory_only_inside_init() {
    let message = [
        declare("a", "local"),
        expect("a", "isInMemory", false),
        change("init", &["1"]),
        expect("a", "isInMemory", true),
        change("init", &["0"]),
        expect("a", "isInMemory", false),
    ]
    .concat();
    assert_eq!(run(&message), "ok\nok\nok\n");
}

#[test]
fn copy_creates_an_owner_and_leaves_the_source_alone() {
    let message = [
        declare("a", "now"),
        change("copy", &["b", "a"]),
        declare("b", "now"),
        expect("a", "isOwned", true),
        expect("b", "isOwned", true),
        expect("a", "isMoved", false),
    ]
    .concat();
    assert_eq!(run(&message), "ok\nok\nok\n");
}

#[test]
fn move_hands_the_instance_over() {
    let message = [
        declare("a", "now"),
        change("move", &["b", "a"]),
        declare("b", "now"),
        change("moved", &["a"]),
        expect("a", "isMoved", true),
        expect("a", "isOwned", false),
        expect("a", "isInMemory", false),
        expect("b", "isOwned", true),
    ]
    .concat();
    assert_eq!(run(&message), "ok\nok\nok\nok\n");
}

#[test]
fn a_borrow_is_a_reference_the_owner_counts() {
    let message = [
        declare("a", "now"),
        change("borrow", &["r", "a"]),
        declare("r", "local"),
        expect("a", "isBorrowed", true),
        expect("r", "isReference", true),
        expect("r", "isOwned", false),
        change("destroyed", &["r"]),
        expect("a", "isBorrowed", false),
        expect("a", "isDestroyed", false),
        expect("r", "isDestroyed", true),
    ]
    .concat();
    assert_eq!(run(&message), "ok\nok\nok\nok\nok\nok\n");
}

#[test]
fn destroy_ends_the_instance() {
    let message = [
        declare("a", "now"),
        change("destroyed", &["a"]),
        expect("a", "isDestroyed", true),
        expect("a", "isOwned", false),
        expect("a", "isInMemory", false),
    ]
    .concat();
    assert_eq!(run(&message), "ok\nok\nok\n");
}

#[test]
fn a_failed_expectation_throws_its_message() {
    let message = [declare("a", "now"), expect("a", "isMoved", true)].concat();
    assert!(run_error(&message).contains("Into assertion failed: fail"));
}

#[test]
fn asking_about_an_unknown_binding_is_an_error() {
    let message = expect("missing", "isOwned", true);
    assert!(run_error(&message).contains("`missing` has no binding"));
}
