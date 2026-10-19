mod support;

use support::{record, run};
use types::MessageTag::*;

#[test]
fn text_lines_are_printed_verbatim() {
    assert_eq!(run("hello\nworld\n"), "hello\nworld\n");
}

#[test]
fn a_while_loop_counts_down() {
    let message = [
        record(Assign, &["n", "Int32", "n:3"]),
        record(WhileBegin, &["n", ">", "n:0", "Int32"]),
        record(Print, &["n", "Int32", "1"]),
        record(Arith, &["n", "n", "sub", "n:1", "Int32"]),
        record(LoopEnd, &[]),
    ]
    .concat();
    assert_eq!(run(&message), "3\n2\n1\n");
}

#[test]
fn break_leaves_the_innermost_loop() {
    let message = [
        record(Assign, &["n", "Int32", "n:0"]),
        record(WhileBegin, &["n", "<", "n:10", "Int32"]),
        record(Arith, &["n", "n", "add", "n:1", "Int32"]),
        record(IfBegin, &["n", "==", "n:3", "Int32"]),
        record(Break, &[]),
        record(IfEnd, &[]),
        record(LoopEnd, &[]),
        record(Print, &["n", "Int32", "1"]),
    ]
    .concat();
    assert_eq!(run(&message), "3\n");
}

#[test]
fn if_runs_the_matching_branch() {
    let branch = |value: &str| {
        [
            record(Assign, &["n", "Int32", value]),
            record(IfBegin, &["n", "==", "n:1", "Int32"]),
            "then\n".to_owned(),
            record(IfElse, &[]),
            "else\n".to_owned(),
            record(IfEnd, &[]),
            "after\n".to_owned(),
        ]
        .concat()
    };
    assert_eq!(run(&branch("n:1")), "then\nafter\n");
    assert_eq!(run(&branch("n:2")), "else\nafter\n");
}

#[test]
fn an_if_without_else_is_skipped() {
    let message = [
        record(IfConst, &["0"]),
        "hidden\n".to_owned(),
        record(IfEnd, &[]),
        "shown\n".to_owned(),
    ]
    .concat();
    assert_eq!(run(&message), "shown\n");
}

#[test]
fn integers_wrap_at_their_width() {
    let message = [
        record(Assign, &["n", "Int8", "n:127"]),
        record(Arith, &["n", "n", "add", "n:1", "Int8"]),
        record(Print, &["n", "Int8", "1"]),
    ]
    .concat();
    assert_eq!(run(&message), "-128\n");
}
