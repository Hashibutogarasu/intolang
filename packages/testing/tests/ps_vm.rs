#![cfg(feature = "run")]

use intovm::{ProcessState, VmOptions};
use testing::eval_vm_bundled;

#[test]
fn the_current_process_reports_the_virtual_pid() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
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
fn the_current_process_is_unique() {
    let error = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
now into again = Process.current();
println(again.pid);
"#,
        VmOptions::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("holds no instance"), "{error}");
}

#[test]
fn signals_sent_to_the_process_reach_their_handlers_when_dispatched() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
self.signal.on(Signal.SIGUSR1, (signal) => { println("usr1"); });
self.signal.on(Signal.SIGUSR2, (signal) => { println("usr2"); });
self.signal.send(Signal.SIGUSR1);
dispatch(self);
println("done");
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"usr1
done
"#
    );
}

#[test]
fn a_dispatch_without_signals_calls_nothing() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
self.signal.on(Signal.SIGUSR1, (signal) => { println("usr1"); });
dispatch(self);
println("quiet");
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"quiet
"#
    );
}

#[test]
fn a_destroyed_listener_no_longer_hears_the_signal() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
now into listener = self.signal.on(Signal.SIGUSR1, (signal) => { println("usr1"); });
listener.destroy();
self.signal.send(Signal.SIGUSR1);
dispatch(self);
println("done");
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"done
"#
    );
}

#[test]
fn exit_fires_exited_and_ends_the_program_with_the_code_after_the_scopes() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
exited.on((process, code) => { println(code); });
exit(self, 3);
println("still running");
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"3
still running
"#
    );
    assert_eq!(outcome.exit(), Some(ProcessState::Exited(3)));
}

#[test]
fn destroying_the_current_process_does_not_end_it() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
self.destroy();
println("alive");
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"alive
"#
    );
    assert_eq!(outcome.exit(), Some(ProcessState::Exited(0)));
}

#[test]
fn passing_the_process_by_value_destroys_it_without_ending_the_program() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
pub fun adopt(owner: Process) { println("adopted"); }
adopt(self.move());
println("still running");
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"adopted
still running
"#
    );
    assert_eq!(outcome.exit(), Some(ProcessState::Exited(0)));
}

#[test]
fn wait_returns_the_exit_code_of_a_child() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into ok = Process.spawn("/usr/bin/env", "true");
println(ok.wait());
now into bad = Process.spawn("/usr/bin/env", "false");
now into code = bad.wait();
println(code);
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"0
1
"#
    );
}

#[test]
fn wait_fires_the_exited_event() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

exited.on((process, code) => { println("exited"); println(code); });
now into child = Process.spawn("/usr/bin/env", "true");
now into code = child.wait();
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"exited
0
"#
    );
}

#[test]
fn kill_ends_a_child_without_starting_anything_real() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into child = Process.spawn("/usr/bin/env", "sleep 60");
child.kill();
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
    assert_eq!(outcome.clock_ms, 0);
}

#[test]
fn waiting_for_a_sleeping_child_takes_virtual_time_only() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into child = Process.spawn("/usr/bin/env", "sleep 60");
now into code = child.wait();
println(code);
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"0
"#
    );
    assert_eq!(outcome.clock_ms, 60_000);
}

#[test]
fn a_reference_parameter_may_kill_without_owning() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

pub fun stop(target: Process&) { target.kill(); }
now into child = Process.spawn("/usr/bin/env", "sleep 60");
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
fn a_reference_cannot_detach() {
    let error = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

pub fun release(target: Process&) { target.detach(); }
now into child = Process.spawn("/usr/bin/env", "sleep 60");
release(child);
"#,
        VmOptions::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("requires ownership"), "{error}");
}

#[test]
fn a_detached_child_cannot_be_used_again() {
    let error = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into child = Process.spawn("/usr/bin/env", "true");
child.detach();
child.wait();
"#,
        VmOptions::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`child` was detached"), "{error}");
}

#[test]
fn a_child_keeps_running_after_its_handle_is_destroyed() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into child = Process.spawn("/usr/bin/env", "sleep 60");
child.destroy();
println("released");
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"released
"#
    );
    let child = outcome.process(4243).unwrap();
    assert_eq!(child.state, ProcessState::Running);
}

#[test]
fn killing_the_current_process_ends_the_program_at_once() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
self.kill();
println("unreachable");
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(outcome.stdout, "");
    assert_eq!(outcome.exit(), Some(ProcessState::Signaled(9)));
}

#[test]
fn usage_reports_the_resources_of_the_process() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
now into usage = self.usage();
println(usage.maxResidentKilobytes);
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        outcome.stdout,
        r#"1024
"#
    );
}

#[test]
fn another_process_can_be_attached_by_pid() {
    let outcome = eval_vm_bundled(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into other = Process.attach(1);
println(other.pid);
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
