#![cfg(feature = "run")]

use testing::eval_run;

#[test]
fn the_current_process_reports_its_pid() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
println(self.pid);
"#,
    )
    .unwrap();
    assert!(
        execution.success || execution.code.is_some(),
        "{}",
        execution.stderr
    );
    let pid: i32 = execution.stdout.trim().parse().unwrap();
    assert!(pid > 0, "{}", execution.stdout);
}

#[test]
fn the_current_process_is_unique() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
now into again = Process.current();
println(again.pid);
"#,
    )
    .unwrap();
    assert!(!execution.success, "{}", execution.stdout);
}

#[test]
fn signals_sent_to_the_process_reach_their_handlers_when_dispatched() {
    let execution = eval_run(
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
    )
    .unwrap();
    assert!(
        execution.success || execution.code.is_some(),
        "{}",
        execution.stderr
    );
    assert_eq!(
        execution.stdout,
        r#"usr1
done
"#
    );
}

#[test]
fn a_dispatch_without_signals_calls_nothing() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
self.signal.on(Signal.SIGUSR1, (signal) => { println("usr1"); });
dispatch(self);
println("quiet");
"#,
    )
    .unwrap();
    assert!(
        execution.success || execution.code.is_some(),
        "{}",
        execution.stderr
    );
    assert_eq!(
        execution.stdout,
        r#"quiet
"#
    );
}

#[test]
fn a_destroyed_listener_no_longer_hears_the_signal() {
    let execution = eval_run(
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
    )
    .unwrap();
    assert!(
        execution.success || execution.code.is_some(),
        "{}",
        execution.stderr
    );
    assert_eq!(
        execution.stdout,
        r#"done
"#
    );
}

#[test]
fn exit_fires_exited_and_ends_the_program_with_the_code_after_the_scopes() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
exited.on((process, code) => { println(code); });
exit(self, 3);
println("still running");
"#,
    )
    .unwrap();
    assert_eq!(
        execution.stdout,
        r#"3
still running
"#
    );
    assert_eq!(execution.code, Some(3));
}

#[test]
fn destroying_the_current_process_does_not_end_it() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
self.destroy();
println("alive");
"#,
    )
    .unwrap();
    assert_eq!(
        execution.stdout,
        r#"alive
"#
    );
    assert_eq!(execution.code, Some(0));
}

#[test]
fn passing_the_process_by_value_destroys_it_without_ending_the_program() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
pub fun adopt(owner: Process) { println("adopted"); }
adopt(self.move());
println("still running");
"#,
    )
    .unwrap();
    assert_eq!(
        execution.stdout,
        r#"adopted
still running
"#
    );
    assert_eq!(execution.code, Some(0));
}

#[test]
fn wait_returns_the_exit_code_of_a_child() {
    let execution = eval_run(
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
    )
    .unwrap();
    assert_eq!(
        execution.stdout,
        r#"0
1
"#
    );
}

#[test]
fn wait_fires_the_exited_event() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

exited.on((process, code) => { println("exited"); println(code); });
now into child = Process.spawn("/usr/bin/env", "true");
now into code = child.wait();
"#,
    )
    .unwrap();
    assert_eq!(
        execution.stdout,
        r#"exited
0
"#
    );
}

#[test]
fn kill_ends_a_child_and_wait_reports_the_signal() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into child = Process.spawn("/usr/bin/env", "sleep 60");
child.kill();
now into code = child.wait();
println(code);
"#,
    )
    .unwrap();
    assert_eq!(
        execution.stdout,
        r#"-9
"#
    );
}

#[test]
fn a_reference_parameter_may_kill_without_owning() {
    let execution = eval_run(
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
    )
    .unwrap();
    assert_eq!(
        execution.stdout,
        r#"-9
"#
    );
}

#[test]
fn a_reference_cannot_detach() {
    let error = eval_run(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

pub fun release(target: Process&) { target.detach(); }
now into child = Process.spawn("/usr/bin/env", "sleep 60");
release(child);
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("requires ownership"), "{error}");
}

#[test]
fn a_detached_child_cannot_be_used_again() {
    let error = eval_run(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into child = Process.spawn("/usr/bin/env", "true");
child.detach();
child.wait();
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`child` was detached"), "{error}");
}

#[test]
fn a_child_keeps_running_after_its_handle_is_destroyed() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into child = Process.spawn("/usr/bin/env", "true");
child.destroy();
println("released");
"#,
    )
    .unwrap();
    assert_eq!(
        execution.stdout,
        r#"released
"#
    );
}

#[test]
fn usage_reports_the_resources_of_the_process() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into self = Process.current();
now into usage = self.usage();
println(usage.maxResidentKilobytes);
"#,
    )
    .unwrap();
    let kilobytes: i64 = execution.stdout.trim().parse().unwrap();
    assert!(kilobytes > 0, "{}", execution.stdout);
}

#[test]
fn another_process_can_be_attached_by_pid() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";
import "ps://process";
import "ps://events";

now into other = Process.attach(1);
println(other.pid);
"#,
    )
    .unwrap();
    assert_eq!(
        execution.stdout,
        r#"1
"#
    );
}
