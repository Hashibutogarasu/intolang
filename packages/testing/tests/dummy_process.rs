#![cfg(feature = "run")]

use intovm::{ProcessState, VmOptions};
use testing::{eval_run, eval_vm_bundled};

#[test]
fn a_dummy_child_is_killed_on_the_virtual_table_only() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "intolang://builtin/tests/dummy-process";
import "ps://process";
import "ps://events";

now into child = DummyProcess.spawn("/bin/sleep", "60");
child.kill();
now into code = child.wait();
println(code);
println(DummyProcess.SIGNALED);
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"-9
-9
"#
    );
    assert_eq!(
        outcome.process(4243).map(|process| process.state),
        Some(ProcessState::Signaled(9))
    );
}

#[test]
fn the_dummy_current_process_is_the_virtual_one() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "intolang://builtin/tests/dummy-process";
import "ps://process";
import "ps://events";

now into self = DummyProcess.current();
println(self.pid);
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"4242
"#
    );
}

#[test]
fn destroying_the_dummy_current_process_does_not_end_it() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "intolang://builtin/tests/dummy-process";
import "ps://process";
import "ps://events";

now into self = DummyProcess.current();
self.destroy();
now into alive = DummyProcess.isAlive(4242);
println(alive);
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"1
"#
    );
}

#[test]
fn is_alive_follows_the_life_of_a_dummy_child() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "intolang://builtin/tests/dummy-process";
import "ps://process";
import "ps://events";

now into child = DummyProcess.spawn("/bin/sleep", "2");
now into before = DummyProcess.isAlive(4243);
println(before);
DummyProcess.advance(2000);
now into after = DummyProcess.isAlive(4243);
println(after);
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"1
0
"#
    );
    assert_eq!(outcome.clock_ms, 2000);
}

#[test]
fn a_dummy_process_is_still_a_process() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "intolang://builtin/tests/dummy-process";
import "ps://process";
import "ps://events";

pub fun stop(target: Process&) { target.kill(); }
now into child = DummyProcess.spawn("/bin/sleep", "60");
stop(child);
now into code = child.wait();
println(code);
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"-9
"#
    );
}

#[test]
fn compiled_code_cannot_use_a_dummy_process() {
    let error = eval_run(
        r#"
import "intolang://builtin/std";
import "intolang://builtin/tests/dummy-process";
import "ps://process";
import "ps://events";

now into child = DummyProcess.spawn("/bin/true", "");
println(child.pid);
"#,
    )
    .unwrap_err();
    assert!(
        format!("{error:#}").contains("DummyProcess requires the Into VM"),
        "{error:#}"
    );
}

#[test]
fn attaching_to_pid_zero_gives_no_handle() {
    let error = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "intolang://builtin/tests/dummy-process";
import "ps://process";
import "ps://events";

now into other = Process.attach(0);
println(other.pid);
"#,
        VmOptions::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("holds no instance"), "{error}");
}
