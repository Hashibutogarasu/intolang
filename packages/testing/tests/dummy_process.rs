#![cfg(feature = "run")]

use intovm::{ProcessState, VmOptions};
use testing::{VmOutcome, eval_run, eval_vm_bundled};

const IMPORTS: &str = "import \"intolang://builtin/std\";\nimport \"intolang://builtin/tests/dummy-process\";\nimport \"ps://process\";\nimport \"ps://events\";\n";

fn run(body: &str) -> VmOutcome {
    eval_vm_bundled(&format!("{IMPORTS}{body}"), VmOptions::default()).unwrap()
}

fn error_of(body: &str) -> String {
    eval_vm_bundled(&format!("{IMPORTS}{body}"), VmOptions::default())
        .unwrap_err()
        .to_string()
}

#[test]
fn a_dummy_child_is_killed_on_the_virtual_table_only() {
    let outcome = run(
        "now into child = DummyProcess.spawn(\"/bin/sleep\", \"60\");\nchild.kill();\nnow into code = child.wait();\nprintln(code);\nprintln(DummyProcess.SIGNALED);\n",
    );
    assert_eq!(outcome.stdout, "-9\n-9\n");
    assert_eq!(
        outcome.process(4243).map(|process| process.state),
        Some(ProcessState::Signaled(9))
    );
}

#[test]
fn the_dummy_current_process_is_the_virtual_one() {
    let outcome = run("now into self = DummyProcess.current();\nprintln(self.pid);\n");
    assert_eq!(outcome.stdout, "4242\n");
}

#[test]
fn destroying_the_dummy_current_process_does_not_end_it() {
    let outcome = run(
        "now into self = DummyProcess.current();\nself.destroy();\nnow into alive = DummyProcess.isAlive(4242);\nprintln(alive);\n",
    );
    assert_eq!(outcome.stdout, "1\n");
}

#[test]
fn is_alive_follows_the_life_of_a_dummy_child() {
    let outcome = run(
        "now into child = DummyProcess.spawn(\"/bin/sleep\", \"2\");\nnow into before = DummyProcess.isAlive(4243);\nprintln(before);\nDummyProcess.advance(2000);\nnow into after = DummyProcess.isAlive(4243);\nprintln(after);\n",
    );
    assert_eq!(outcome.stdout, "1\n0\n");
    assert_eq!(outcome.clock_ms, 2000);
}

#[test]
fn a_dummy_process_is_still_a_process() {
    let outcome = run(
        "pub fun stop(target: Process&) { target.kill(); }\nnow into child = DummyProcess.spawn(\"/bin/sleep\", \"60\");\nstop(child);\nnow into code = child.wait();\nprintln(code);\n",
    );
    assert_eq!(outcome.stdout, "-9\n");
}

#[test]
fn compiled_code_cannot_use_a_dummy_process() {
    let source = format!(
        "{IMPORTS}now into child = DummyProcess.spawn(\"/bin/true\", \"\");\nprintln(child.pid);\n"
    );
    let error = eval_run(&source).unwrap_err();
    assert!(
        format!("{error:#}").contains("DummyProcess requires the Into VM"),
        "{error:#}"
    );
}

#[test]
fn attaching_to_pid_zero_gives_no_handle() {
    let error = error_of("now into other = Process.attach(0);\nprintln(other.pid);\n");
    assert!(error.contains("holds no instance"), "{error}");
}
