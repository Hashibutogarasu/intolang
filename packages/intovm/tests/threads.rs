mod support;

use support::{record, run, run_error};
use types::MessageTag::*;

fn function(name: &str, captures: &str, body: String) -> String {
    [
        record(HandlerBegin, &[name, captures]),
        body,
        record(HandlerEnd, &[]),
    ]
    .concat()
}

#[test]
fn a_thread_runs_only_when_the_main_thread_waits() {
    let message = [
        function("work", "", "work\n".to_owned()),
        record(ThreadRun, &["task", "work"]),
        "main\n".to_owned(),
        record(Await, &["task"]),
        "joined\n".to_owned(),
    ]
    .concat();
    assert_eq!(run(&message), "main\nwork\njoined\n");
}

#[test]
fn a_thread_that_is_never_awaited_does_not_run() {
    let message = [
        function("work", "", "work\n".to_owned()),
        record(ThreadRun, &["", "work"]),
        "main\n".to_owned(),
    ]
    .concat();
    assert_eq!(run(&message), "main\n");
}

#[test]
fn threads_take_turns_in_creation_order() {
    let message = [
        function("a", "", "a\n".to_owned()),
        function("b", "", "b\n".to_owned()),
        record(ThreadRun, &["first", "a"]),
        record(ThreadRun, &["second", "b"]),
        record(Await, &["second"]),
        record(Await, &["first"]),
        "end\n".to_owned(),
    ]
    .concat();
    assert_eq!(run(&message), "a\nb\nend\n");
}

#[test]
fn a_thread_can_wait_for_another() {
    let message = [
        function("inner", "", "inner\n".to_owned()),
        function(
            "outer",
            "inner_task",
            [record(Await, &["inner_task"]), "outer\n".to_owned()].concat(),
        ),
        record(ThreadRun, &["inner_task", "inner"]),
        record(ThreadRun, &["outer_task", "outer"]),
        record(Await, &["outer_task"]),
    ]
    .concat();
    assert_eq!(run(&message), "inner\nouter\n");
}

#[test]
fn threads_share_captured_slots() {
    let message = [
        record(Assign, &["count", "Int32", "n:0"]),
        function(
            "bump",
            "count",
            record(Arith, &["count", "count", "add", "n:5", "Int32"]),
        ),
        record(ThreadRun, &["task", "bump"]),
        record(Await, &["task"]),
        record(Print, &["count", "Int32", "1"]),
    ]
    .concat();
    assert_eq!(run(&message), "5\n");
}

#[test]
fn two_threads_waiting_for_each_other_is_a_deadlock() {
    let message = [
        function("a", "second", record(Await, &["second"])),
        function("b", "first", record(Await, &["first"])),
        record(ThreadRun, &["first", "a"]),
        record(ThreadRun, &["second", "b"]),
        record(Await, &["first"]),
    ]
    .concat();
    assert!(run_error(&message).contains("deadlock"));
}
