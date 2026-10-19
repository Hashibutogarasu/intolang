mod support;

use intovm::{ChildBehavior, ProcessState, VmOptions};
use std::collections::HashMap;
use support::{ps_call, record, run_error, text, vm_of};
use types::MessageTag::*;

const PROCESS: &str = "Process";

fn classes() -> String {
    [
        record(Class, &["Process", "pid:Int32:n:0"]),
        record(Class, &["Signal"]),
        record(
            Class,
            &[
                "Usage",
                "userMicros:Int64:n:0",
                "systemMicros:Int64:n:0",
                "maxResidentKilobytes:Int64:n:0",
            ],
        ),
    ]
    .concat()
}

fn spawn(name: &str, path: &str, args: &str) -> String {
    ps_call(
        "ps_process_spawn",
        PROCESS,
        name,
        &[&text(path), &text(args)],
    )
}

fn print(name: &str) -> String {
    record(Print, &[name, "Int32", "1"])
}

fn finished(message: &str, options: VmOptions) -> intovm::Vm {
    let mut vm = vm_of(message, options);
    vm.run().unwrap();
    vm
}

#[test]
fn the_current_process_has_the_configured_pid() {
    let message = [
        classes(),
        ps_call("ps_process_current", PROCESS, "me", &[]),
        record(FieldPrint, &["me", PROCESS, "0", "Int32", "1"]),
    ]
    .concat();
    let options = VmOptions {
        current_pid: 77,
        ..VmOptions::default()
    };
    assert_eq!(finished(&message, options).stdout(), "77\n");
}

#[test]
fn the_current_process_can_only_be_taken_once_until_it_is_released() {
    let message = [
        classes(),
        ps_call("ps_process_current", PROCESS, "me", &[]),
        ps_call("ps_process_current", PROCESS, "again", &[]),
        record(FieldPrint, &["again", PROCESS, "0", "Int32", "1"]),
    ]
    .concat();
    assert!(run_error(&message).contains("holds no instance"));
    let message = [
        classes(),
        ps_call("ps_process_current", PROCESS, "me", &[]),
        ps_call("ps_process_release", "Void", "", &["p:me"]),
        ps_call("ps_process_current", PROCESS, "again", &[]),
        record(FieldPrint, &["again", PROCESS, "0", "Int32", "1"]),
    ]
    .concat();
    assert_eq!(finished(&message, VmOptions::default()).stdout(), "4242\n");
}

#[test]
fn wait_returns_the_exit_code_of_a_child() {
    let message = [
        classes(),
        spawn("ok", "/bin/true", ""),
        spawn("bad", "/bin/false", ""),
        ps_call("ps_process_reap", "Int32", "okCode", &["p:ok"]),
        ps_call("ps_process_reap", "Int32", "badCode", &["p:bad"]),
        print("okCode"),
        print("badCode"),
    ]
    .concat();
    assert_eq!(finished(&message, VmOptions::default()).stdout(), "0\n1\n");
}

#[test]
fn waiting_for_a_sleeping_child_moves_the_virtual_clock() {
    let message = [
        classes(),
        spawn("child", "/bin/sleep", "60"),
        ps_call("ps_process_reap", "Int32", "code", &["p:child"]),
        print("code"),
    ]
    .concat();
    let vm = finished(&message, VmOptions::default());
    assert_eq!(vm.stdout(), "0\n");
    assert_eq!(vm.processes().clock(), 60_000);
}

#[test]
fn kill_ends_a_long_running_child_and_wait_reports_the_signal() {
    let message = [
        classes(),
        spawn("child", "/bin/sleep", "3600"),
        ps_call("ps_process_kill", "Int32", "killed", &["p:child"]),
        ps_call("ps_process_reap", "Int32", "code", &["p:child"]),
        print("killed"),
        print("code"),
    ]
    .concat();
    let vm = finished(&message, VmOptions::default());
    assert_eq!(vm.stdout(), "0\n-9\n");
    assert_eq!(vm.processes().clock(), 0);
}

#[test]
fn waiting_for_a_program_that_never_ends_is_a_deadlock() {
    let message = [
        classes(),
        spawn("child", "/bin/sleep", ""),
        ps_call("ps_process_reap", "Int32", "code", &["p:child"]),
    ]
    .concat();
    assert!(run_error(&message).contains("deadlock"));
}

#[test]
fn programs_can_be_scripted() {
    let message = [
        classes(),
        spawn("child", "/opt/job", "1"),
        ps_call("ps_process_reap", "Int32", "code", &["p:child"]),
        print("code"),
    ]
    .concat();
    let options = VmOptions {
        programs: HashMap::from([(
            "/opt/job".to_owned(),
            ChildBehavior {
                code: 3,
                runs_for_ms: Some(250),
            },
        )]),
        ..VmOptions::default()
    };
    let vm = finished(&message, options);
    assert_eq!(vm.stdout(), "3\n");
    assert_eq!(vm.processes().clock(), 250);
}

#[test]
fn releasing_a_handle_leaves_the_child_running() {
    let message = [
        classes(),
        spawn("child", "/bin/sleep", "3600"),
        ps_call("ps_process_release", "Void", "", &["p:child"]),
    ]
    .concat();
    let vm = finished(&message, VmOptions::default());
    let child = vm.processes().get(4243).unwrap();
    assert_eq!(child.state, ProcessState::Running);
    assert!(!child.reaped);
}

#[test]
fn a_reaped_child_is_gone() {
    let message = [
        classes(),
        spawn("child", "/bin/true", ""),
        ps_call("ps_process_reap", "Int32", "code", &["p:child"]),
        ps_call("ps_process_kill", "Int32", "late", &["p:child"]),
        print("late"),
    ]
    .concat();
    assert_eq!(finished(&message, VmOptions::default()).stdout(), "-1\n");
}

#[test]
fn watched_signals_are_queued_and_unwatched_ones_end_the_process() {
    let send = |number: &str| {
        [
            ps_call("ps_process_current", PROCESS, "me", &[]),
            ps_call("ps_process_signal", "Signal", "line", &["p:me"]),
            ps_call(
                "ps_signal_send",
                "Int32",
                "sent",
                &["p:line", &format!("i:{number}")],
            ),
        ]
        .concat()
    };
    let watched = [
        classes(),
        ps_call("ps_signal_watch", "Void", "", &["i:15"]),
        send("15"),
        ps_call("ps_signal_next", "Int32", "first", &[]),
        ps_call("ps_signal_next", "Int32", "second", &[]),
        print("first"),
        print("second"),
    ]
    .concat();
    assert_eq!(finished(&watched, VmOptions::default()).stdout(), "15\n0\n");

    let unwatched = [classes(), send("15"), "after\n".to_owned()].concat();
    let vm = finished(&unwatched, VmOptions::default());
    assert_eq!(vm.stdout(), "");
    assert_eq!(
        vm.processes().get(4242).unwrap().state,
        ProcessState::Signaled(15)
    );
}

#[test]
fn kill_cannot_be_watched() {
    let message = [
        classes(),
        ps_call("ps_signal_watch", "Void", "", &["i:9"]),
        ps_call("ps_process_current", PROCESS, "me", &[]),
        ps_call("ps_process_kill", "Int32", "code", &["p:me"]),
        "after\n".to_owned(),
    ]
    .concat();
    let vm = finished(&message, VmOptions::default());
    assert_eq!(vm.stdout(), "");
    assert_eq!(
        vm.processes().get(4242).unwrap().state,
        ProcessState::Signaled(9)
    );
}

#[test]
fn a_signal_from_outside_ends_a_process_that_does_not_watch_it() {
    let mut vm = vm_of(&classes(), VmOptions::default());
    assert_eq!(vm.send_signal(4242, 2), 0);
    assert_eq!(vm.send_signal(9999, 2), -1);
    vm.run().unwrap();
    assert_eq!(
        vm.processes().get(4242).unwrap().state,
        ProcessState::Signaled(2)
    );
}

#[test]
fn the_exit_request_sets_the_exit_code_after_the_program_ended() {
    let message = [classes(), ps_call("ps_exit_request", "Void", "", &["i:7"])].concat();
    let vm = finished(&message, VmOptions::default());
    assert_eq!(
        vm.processes().get(4242).unwrap().state,
        ProcessState::Exited(7)
    );
}

#[test]
fn usage_follows_the_virtual_clock() {
    let message = [
        classes(),
        ps_call("ps_process_current", PROCESS, "me", &[]),
        ps_call("ps_process_usage", "Usage", "usage", &["p:me"]),
        record(FieldPrint, &["usage", "Usage", "0", "Int64", "1"]),
    ]
    .concat();
    let options = VmOptions {
        clock_ms: 5,
        ..VmOptions::default()
    };
    assert_eq!(finished(&message, options).stdout(), "5000\n");
}
