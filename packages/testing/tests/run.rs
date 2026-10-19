#![cfg(feature = "run")]

use testing::eval_run;

#[test]
fn runs_a_compiled_program() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";

init {
    into index: Int32 = 0;
    while (index < 3) {
        println(index);
        index = index + 1;
    }
}
"#,
    )
    .unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert_eq!(
        execution.stdout,
        r#"0
1
2
"#
    );
}

#[test]
fn prints_compile_time_text() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";

init {
    println("hello");
}
"#,
    )
    .unwrap();
    assert_eq!(
        execution.stdout,
        r#"hello
"#
    );
}

#[test]
fn compiles_scope_end_destroys_with_drop_flags() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
    pub override fun destroy(): Void { println("released"); }
}

init {
    now into a = Counter();
    into index: Int32 = 0;
    while (index < 2) {
        now into inner = Counter();
        if (index == 1) {
            now into taken = a.move();
        }
        index = index + 1;
    }
    println(index);
}
"#,
    )
    .unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert!(execution.stdout.contains("2\n"), "{}", execution.stdout);
}

#[test]
fn objects_have_a_default_text_and_hash() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 value = 7;
}

init {
    now into a = Counter();
    now into b = a.copy();
    println(a.toString());
    now into first = a.hash();
    now into second = a.hash();
    now into other = b.hash();
    if (first == second) {
        println("same");
    }
    if (first != other) {
        println("different");
    }
}
"#,
    )
    .unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert!(
        execution.stdout.starts_with("Object@"),
        "{}",
        execution.stdout
    );
    assert!(execution.stdout.contains("same\n"), "{}", execution.stdout);
    assert!(
        execution.stdout.contains("different\n"),
        "{}",
        execution.stdout
    );
}

#[test]
fn emit_calls_every_registered_handler_in_order() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
example.on((value) => { println(value); });
example.emit(3);
example.emit(4);
now into listener = example.on((value) => { println("second"); });
example.emit(5);
listener.destroy();
example.emit(6);
"#,
    )
    .unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert_eq!(
        execution.stdout,
        r#"3
4
5
second
6
"#
    );
}

#[test]
fn destroying_an_event_silences_its_handlers() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
example.on((value) => { println(value); });
example.emit(1);
example.destroy();
"#,
    )
    .unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert_eq!(
        execution.stdout,
        r#"1
"#
    );
}

#[test]
fn the_default_handler_runs_before_the_registered_ones() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";

pub event tick(value: Int32) {
    println("default");
}
now into total = 10;
tick.on((value) => { println(total); });
tick.emit(1);
"#,
    )
    .unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert_eq!(
        execution.stdout,
        r#"default
10
"#
    );
}

#[test]
fn handlers_receive_string_arguments_and_see_later_changes_of_captures() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";

pub event greet(name: String) {}
now into count = 1;
greet.on((name) => { println(name); println(count); });
count = 2;
greet.emit("bob");
"#,
    )
    .unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert_eq!(
        execution.stdout,
        r#"bob
2
"#
    );
}

#[test]
fn a_listener_leaving_its_scope_unregisters_the_handler() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
now into n = 1;
if (n == 1) {
    now into listener = example.on((value) => { println(value); });
    example.emit(1);
}
example.emit(2);
"#,
    )
    .unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert_eq!(
        execution.stdout,
        r#"1
"#
    );
}

#[test]
fn text_inside_a_branch_only_prints_when_the_branch_runs() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";

now into n = 1;
if (n == 2) {
    println("never");
} else {
    println("else");
}
if (n == 1) {
    println("taken");
}
while (n < 3) {
    println("loop");
    n = n + 1;
}
println("after");
"#,
    )
    .unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert_eq!(
        execution.stdout,
        r#"else
taken
loop
loop
after
"#
    );
}

#[test]
fn await_waits_for_the_thread_before_the_next_line() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";

now into worker = thread { println("in thread"); };
await worker.run();
println("after");
"#,
    )
    .unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert_eq!(
        execution.stdout,
        r#"in thread
after
"#
    );
}

#[test]
fn a_task_can_be_awaited_later() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";

now into worker = thread { println("in thread"); };
now into task = worker.run();
await task;
println("done");
"#,
    )
    .unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert_eq!(
        execution.stdout,
        r#"in thread
done
"#
    );
}

#[test]
fn threads_share_captured_values_with_the_main_flow() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";

now into total = 0;
now into worker = thread { total = 5; };
await worker.run();
println(total);
"#,
    )
    .unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert_eq!(
        execution.stdout,
        r#"5
"#
    );
}

#[test]
fn several_threads_run_and_join() {
    let execution = eval_run(
        r#"
import "intolang://builtin/std";

now into first = thread { println("one"); };
now into second = thread { println("two"); };
now into a = first.run();
now into b = second.run();
await a;
await b;
println("joined");
"#,
    )
    .unwrap();
    assert!(execution.success, "{}", execution.stderr);
    let stdout = execution.stdout;
    assert!(stdout.contains("one\n"), "{stdout}");
    assert!(stdout.contains("two\n"), "{stdout}");
    assert!(stdout.ends_with("joined\n"), "{stdout}");
}

#[test]
fn reports_lowering_errors_before_compiling() {
    let error = eval_run(
        r#"
init {
    into value: Int32 = "x";
}
"#,
    )
    .unwrap_err();
    assert!(error.to_string().contains("Type mismatch"), "{error}");
}
