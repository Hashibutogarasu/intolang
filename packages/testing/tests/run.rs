#![cfg(feature = "run")]

use testing::eval_run;

#[test]
fn runs_a_compiled_program() {
    let source = "import \"intolang://builtin/std\";\ninit {\n    into index: Int32 = 0;\n    while (index < 3) {\n        println(index);\n        index = index + 1;\n    }\n}\n";
    let execution = eval_run(source).unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert_eq!(execution.stdout, "0\n1\n2\n");
}

#[test]
fn prints_compile_time_text() {
    let source = "import \"intolang://builtin/std\";\ninit {\n    println(\"hello\");\n}\n";
    assert_eq!(eval_run(source).unwrap().stdout, "hello\n");
}

#[test]
fn compiles_scope_end_destroys_with_drop_flags() {
    let source = "import \"intolang://builtin/std\";\npub Counter() {\n    pub into: Int32 value = 7;\n    pub override fun destroy(): Void { println(\"released\"); }\n}\ninit {\n    now into a = Counter();\n    into index: Int32 = 0;\n    while (index < 2) {\n        now into inner = Counter();\n        if (index == 1) {\n            now into taken = a.move();\n        }\n        index = index + 1;\n    }\n    println(index);\n}\n";
    let execution = eval_run(source).unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert!(execution.stdout.contains("2\n"), "{}", execution.stdout);
}

#[test]
fn objects_have_a_default_text_and_hash() {
    let source = "import \"intolang://builtin/std\";\npub Counter() {\n    pub into: Int32 value = 7;\n}\ninit {\n    now into a = Counter();\n    now into b = a.copy();\n    println(a.toString());\n    now into first = a.hash();\n    now into second = a.hash();\n    now into other = b.hash();\n    if (first == second) {\n        println(\"same\");\n    }\n    if (first != other) {\n        println(\"different\");\n    }\n}\n";
    let execution = eval_run(source).unwrap();
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

fn run_events(body: &str) -> String {
    let source = format!("import \"intolang://builtin/std\";\n{body}");
    let execution = eval_run(&source).unwrap();
    assert!(execution.success, "{}", execution.stderr);
    execution.stdout
}

#[test]
fn emit_calls_every_registered_handler_in_order() {
    let stdout = run_events(
        "pub event example(value: Int32) {}\nexample.on((value) => { println(value); });\nexample.emit(3);\nexample.emit(4);\nnow into listener = example.on((value) => { println(\"second\"); });\nexample.emit(5);\nlistener.destroy();\nexample.emit(6);\n",
    );
    assert_eq!(stdout, "3\n4\n5\nsecond\n6\n");
}

#[test]
fn destroying_an_event_silences_its_handlers() {
    let stdout = run_events(
        "pub event example(value: Int32) {}\nexample.on((value) => { println(value); });\nexample.emit(1);\nexample.destroy();\n",
    );
    assert_eq!(stdout, "1\n");
}

#[test]
fn the_default_handler_runs_before_the_registered_ones() {
    let stdout = run_events(
        "pub event tick(value: Int32) {\n    println(\"default\");\n}\nnow into total = 10;\ntick.on((value) => { println(total); });\ntick.emit(1);\n",
    );
    assert_eq!(stdout, "default\n10\n");
}

#[test]
fn handlers_receive_string_arguments_and_see_later_changes_of_captures() {
    let stdout = run_events(
        "pub event greet(name: String) {}\nnow into count = 1;\ngreet.on((name) => { println(name); println(count); });\ncount = 2;\ngreet.emit(\"bob\");\n",
    );
    assert_eq!(stdout, "bob\n2\n");
}

#[test]
fn a_listener_leaving_its_scope_unregisters_the_handler() {
    let stdout = run_events(
        "pub event example(value: Int32) {}\nnow into n = 1;\nif (n == 1) {\n    now into listener = example.on((value) => { println(value); });\n    example.emit(1);\n}\nexample.emit(2);\n",
    );
    assert_eq!(stdout, "1\n");
}

#[test]
fn text_inside_a_branch_only_prints_when_the_branch_runs() {
    let stdout = run_events(
        "now into n = 1;\nif (n == 2) {\n    println(\"never\");\n} else {\n    println(\"else\");\n}\nif (n == 1) {\n    println(\"taken\");\n}\nwhile (n < 3) {\n    println(\"loop\");\n    n = n + 1;\n}\nprintln(\"after\");\n",
    );
    assert_eq!(stdout, "else\ntaken\nloop\nloop\nafter\n");
}

#[test]
fn await_waits_for_the_thread_before_the_next_line() {
    let stdout = run_events(
        "now into worker = thread { println(\"in thread\"); };\nawait worker.run();\nprintln(\"after\");\n",
    );
    assert_eq!(stdout, "in thread\nafter\n");
}

#[test]
fn a_task_can_be_awaited_later() {
    let stdout = run_events(
        "now into worker = thread { println(\"in thread\"); };\nnow into task = worker.run();\nawait task;\nprintln(\"done\");\n",
    );
    assert_eq!(stdout, "in thread\ndone\n");
}

#[test]
fn threads_share_captured_values_with_the_main_flow() {
    let stdout = run_events(
        "now into total = 0;\nnow into worker = thread { total = 5; };\nawait worker.run();\nprintln(total);\n",
    );
    assert_eq!(stdout, "5\n");
}

#[test]
fn several_threads_run_and_join() {
    let stdout = run_events(
        "now into first = thread { println(\"one\"); };\nnow into second = thread { println(\"two\"); };\nnow into a = first.run();\nnow into b = second.run();\nawait a;\nawait b;\nprintln(\"joined\");\n",
    );
    assert!(stdout.contains("one\n"), "{stdout}");
    assert!(stdout.contains("two\n"), "{stdout}");
    assert!(stdout.ends_with("joined\n"), "{stdout}");
}

#[test]
fn reports_lowering_errors_before_compiling() {
    let error = eval_run("init {\n    into value: Int32 = \"x\";\n}\n").unwrap_err();
    assert!(error.to_string().contains("Type mismatch"), "{error}");
}
