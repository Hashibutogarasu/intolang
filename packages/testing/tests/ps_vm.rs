#![cfg(feature = "run")]

use intovm::{ProcessState, VmOptions};
use testing::{VmOutcome, eval_vm_bundled};

const IMPORTS: &str =
    "import \"intolang://builtin/std\";\nimport \"ps://process\";\nimport \"ps://events\";\n";

fn run(body: &str) -> VmOutcome {
    eval_vm_bundled(&format!("{IMPORTS}{body}"), VmOptions::default()).unwrap()
}

fn error_of(body: &str) -> String {
    eval_vm_bundled(&format!("{IMPORTS}{body}"), VmOptions::default())
        .unwrap_err()
        .to_string()
}

#[test]
fn the_current_process_reports_the_virtual_pid() {
    let outcome = run("now into self = Process.current();\nprintln(self.pid);\n");
    assert_eq!(outcome.stdout, "4242\n");
}

#[test]
fn the_current_process_is_unique() {
    let error = error_of(
        "now into self = Process.current();\nnow into again = Process.current();\nprintln(again.pid);\n",
    );
    assert!(error.contains("holds no instance"), "{error}");
}

#[test]
fn signals_sent_to_the_process_reach_their_handlers_when_dispatched() {
    let outcome = run(
        "now into self = Process.current();\nself.signal.on(Signal.SIGUSR1, (signal) => { println(\"usr1\"); });\nself.signal.on(Signal.SIGUSR2, (signal) => { println(\"usr2\"); });\nself.signal.send(Signal.SIGUSR1);\ndispatch(self);\nprintln(\"done\");\n",
    );
    assert_eq!(outcome.stdout, "usr1\ndone\n");
}

#[test]
fn a_dispatch_without_signals_calls_nothing() {
    let outcome = run(
        "now into self = Process.current();\nself.signal.on(Signal.SIGUSR1, (signal) => { println(\"usr1\"); });\ndispatch(self);\nprintln(\"quiet\");\n",
    );
    assert_eq!(outcome.stdout, "quiet\n");
}

#[test]
fn a_destroyed_listener_no_longer_hears_the_signal() {
    let outcome = run(
        "now into self = Process.current();\nnow into listener = self.signal.on(Signal.SIGUSR1, (signal) => { println(\"usr1\"); });\nlistener.destroy();\nself.signal.send(Signal.SIGUSR1);\ndispatch(self);\nprintln(\"done\");\n",
    );
    assert_eq!(outcome.stdout, "done\n");
}

#[test]
fn exit_fires_exited_and_ends_the_program_with_the_code_after_the_scopes() {
    let outcome = run(
        "now into self = Process.current();\nexited.on((process, code) => { println(code); });\nexit(self, 3);\nprintln(\"still running\");\n",
    );
    assert_eq!(outcome.stdout, "3\nstill running\n");
    assert_eq!(outcome.exit(), Some(ProcessState::Exited(3)));
}

#[test]
fn destroying_the_current_process_does_not_end_it() {
    let outcome = run("now into self = Process.current();\nself.destroy();\nprintln(\"alive\");\n");
    assert_eq!(outcome.stdout, "alive\n");
    assert_eq!(outcome.exit(), Some(ProcessState::Exited(0)));
}

#[test]
fn passing_the_process_by_value_destroys_it_without_ending_the_program() {
    let outcome = run(
        "now into self = Process.current();\npub fun adopt(owner: Process) { println(\"adopted\"); }\nadopt(self.move());\nprintln(\"still running\");\n",
    );
    assert_eq!(outcome.stdout, "adopted\nstill running\n");
    assert_eq!(outcome.exit(), Some(ProcessState::Exited(0)));
}

#[test]
fn wait_returns_the_exit_code_of_a_child() {
    let outcome = run(
        "now into ok = Process.spawn(\"/usr/bin/env\", \"true\");\nprintln(ok.wait());\nnow into bad = Process.spawn(\"/usr/bin/env\", \"false\");\nnow into code = bad.wait();\nprintln(code);\n",
    );
    assert_eq!(outcome.stdout, "0\n1\n");
}

#[test]
fn wait_fires_the_exited_event() {
    let outcome = run(
        "exited.on((process, code) => { println(\"exited\"); println(code); });\nnow into child = Process.spawn(\"/usr/bin/env\", \"true\");\nnow into code = child.wait();\n",
    );
    assert_eq!(outcome.stdout, "exited\n0\n");
}

#[test]
fn kill_ends_a_child_without_starting_anything_real() {
    let outcome = run(
        "now into child = Process.spawn(\"/usr/bin/env\", \"sleep 60\");\nchild.kill();\nnow into code = child.wait();\nprintln(code);\n",
    );
    assert_eq!(outcome.stdout, "-9\n");
    assert_eq!(outcome.clock_ms, 0);
}

#[test]
fn waiting_for_a_sleeping_child_takes_virtual_time_only() {
    let outcome = run(
        "now into child = Process.spawn(\"/usr/bin/env\", \"sleep 60\");\nnow into code = child.wait();\nprintln(code);\n",
    );
    assert_eq!(outcome.stdout, "0\n");
    assert_eq!(outcome.clock_ms, 60_000);
}

#[test]
fn a_reference_parameter_may_kill_without_owning() {
    let outcome = run(
        "pub fun stop(target: Process&) { target.kill(); }\nnow into child = Process.spawn(\"/usr/bin/env\", \"sleep 60\");\nstop(child);\nnow into code = child.wait();\nprintln(code);\n",
    );
    assert_eq!(outcome.stdout, "-9\n");
}

#[test]
fn a_reference_cannot_detach() {
    let error = error_of(
        "pub fun release(target: Process&) { target.detach(); }\nnow into child = Process.spawn(\"/usr/bin/env\", \"sleep 60\");\nrelease(child);\n",
    );
    assert!(error.contains("requires ownership"), "{error}");
}

#[test]
fn a_detached_child_cannot_be_used_again() {
    let error = error_of(
        "now into child = Process.spawn(\"/usr/bin/env\", \"true\");\nchild.detach();\nchild.wait();\n",
    );
    assert!(error.contains("`child` was detached"), "{error}");
}

#[test]
fn a_child_keeps_running_after_its_handle_is_destroyed() {
    let outcome = run(
        "now into child = Process.spawn(\"/usr/bin/env\", \"sleep 60\");\nchild.destroy();\nprintln(\"released\");\n",
    );
    assert_eq!(outcome.stdout, "released\n");
    let child = outcome.process(4243).unwrap();
    assert_eq!(child.state, ProcessState::Running);
}

#[test]
fn killing_the_current_process_ends_the_program_at_once() {
    let outcome =
        run("now into self = Process.current();\nself.kill();\nprintln(\"unreachable\");\n");
    assert_eq!(outcome.stdout, "");
    assert_eq!(outcome.exit(), Some(ProcessState::Signaled(9)));
}

#[test]
fn usage_reports_the_resources_of_the_process() {
    let outcome = run(
        "now into self = Process.current();\nnow into usage = self.usage();\nprintln(usage.maxResidentKilobytes);\n",
    );
    assert_eq!(outcome.stdout, "1024\n");
}

#[test]
fn another_process_can_be_attached_by_pid() {
    let outcome = run("now into other = Process.attach(1);\nprintln(other.pid);\n");
    assert_eq!(outcome.stdout, "1\n");
}
