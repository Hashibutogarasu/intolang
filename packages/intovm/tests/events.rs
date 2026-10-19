mod support;

use support::{record, run, run_error};
use types::MessageTag::*;

fn handler(name: &str, captures: &str, params: &[&str], body: String) -> String {
    let mut fields = vec![name, captures];
    fields.extend_from_slice(params);
    [record(HandlerBegin, &fields), body, record(HandlerEnd, &[])].concat()
}

fn printer(name: &str, text: &str) -> String {
    handler(name, "", &[], format!("{text}\n"))
}

#[test]
fn emit_calls_the_handlers_in_registration_order() {
    let message = [
        printer("first", "first"),
        printer("second", "second"),
        record(EventOn, &["ready", "", "first"]),
        record(EventOn, &["ready", "", "second"]),
        record(Emit, &["ready"]),
        "after\n".to_owned(),
    ]
    .concat();
    assert_eq!(run(&message), "first\nsecond\nafter\n");
}

#[test]
fn handlers_receive_the_arguments() {
    let message = [
        handler(
            "show",
            "",
            &["rate:Int32"],
            record(Print, &["rate", "Int32", "1"]),
        ),
        record(EventOn, &["audio", "", "show"]),
        record(Emit, &["audio", "i:44100"]),
    ]
    .concat();
    assert_eq!(run(&message), "44100\n");
}

#[test]
fn handlers_reach_captured_slots() {
    let message = [
        record(Assign, &["count", "Int32", "n:0"]),
        handler(
            "bump",
            "count",
            &[],
            record(Arith, &["count", "count", "add", "n:1", "Int32"]),
        ),
        record(EventOn, &["tick", "", "bump"]),
        record(Emit, &["tick"]),
        record(Emit, &["tick"]),
        record(Print, &["count", "Int32", "1"]),
    ]
    .concat();
    assert_eq!(run(&message), "2\n");
}

#[test]
fn off_removes_one_registration_and_destroy_removes_all() {
    let message = [
        printer("first", "first"),
        printer("second", "second"),
        record(EventOn, &["ready", "listener", "first"]),
        record(EventOn, &["ready", "", "second"]),
        record(EventOff, &["ready", "listener"]),
        record(Emit, &["ready"]),
        record(EventDestroy, &["ready"]),
        record(Emit, &["ready"]),
    ]
    .concat();
    assert_eq!(run(&message), "second\n");
}

#[test]
fn a_handler_registered_while_emitting_waits_for_the_next_emit() {
    let message = [
        printer("late", "late"),
        handler(
            "early",
            "",
            &[],
            [
                "early\n".to_owned(),
                record(EventOn, &["ready", "", "late"]),
            ]
            .concat(),
        ),
        record(EventOn, &["ready", "", "early"]),
        record(Emit, &["ready"]),
        "--\n".to_owned(),
        record(Emit, &["ready"]),
    ]
    .concat();
    assert_eq!(run(&message), "early\n--\nearly\nlate\n");
}

#[test]
fn registering_an_unknown_handler_is_an_error() {
    let message = record(EventOn, &["ready", "", "missing"]);
    assert!(run_error(&message).contains("Unknown event handler `missing`"));
}
