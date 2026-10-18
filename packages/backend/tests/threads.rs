use testing::eval;

const STD: &str = "import \"intolang://builtin/std\";\n";

fn message_of(body: &str) -> String {
    eval(&format!("{STD}{body}")).unwrap().to_string()
}

fn error_of(body: &str) -> String {
    eval(&format!("{STD}{body}")).unwrap_err().to_string()
}

#[test]
fn a_thread_body_becomes_a_function_that_run_starts() {
    let message = message_of(
        "now into worker = thread { println(\"in thread\"); };\nnow into task = worker.run();\nawait task;\n",
    );
    assert!(
        message.contains("NATIVE_HANDLER_BEGIN\t__thread_"),
        "{message}"
    );
    assert!(
        message.contains("NATIVE_THREAD_RUN\ttask\t__thread_"),
        "{message}"
    );
    assert!(message.contains("NATIVE_AWAIT\ttask"), "{message}");
}

#[test]
fn run_without_await_does_not_wait() {
    let message =
        message_of("now into worker = thread { println(\"in thread\"); };\nworker.run();\n");
    assert!(
        message.contains("NATIVE_THREAD_RUN\t\t__thread_"),
        "{message}"
    );
    assert!(!message.contains("NATIVE_AWAIT"), "{message}");
}

#[test]
fn awaiting_a_run_starts_and_joins_in_one_statement() {
    let message =
        message_of("now into worker = thread { println(\"in thread\"); };\nawait worker.run();\n");
    assert!(message.contains("NATIVE_THREAD_RUN\t__task"), "{message}");
    assert!(message.contains("NATIVE_AWAIT\t__task"), "{message}");
}

#[test]
fn threads_capture_the_values_they_mention() {
    let message = message_of(
        "now into total = 1;\nnow into worker = thread { println(total); };\nawait worker.run();\n",
    );
    assert!(
        message.contains("NATIVE_HANDLER_BEGIN\t__thread_"),
        "{message}"
    );
    assert!(message.contains("\ttotal\n"), "{message}");
}

#[test]
fn await_needs_a_task() {
    let error = error_of("now into count = 1;\nawait count;\n");
    assert!(error.contains("`await` expects a Task"), "{error}");
    let error = error_of("now into worker = thread { println(\"x\"); };\nawait worker;\n");
    assert!(error.contains("`await` expects a Task"), "{error}");
}

#[test]
fn await_is_a_statement() {
    let error = error_of(
        "now into worker = thread { println(\"x\"); };\nnow into value = await worker.run();\n",
    );
    assert!(
        error.contains("`await` can only be used as a statement"),
        "{error}"
    );
}

#[test]
fn locals_of_a_thread_are_destroyed_when_it_ends() {
    let message = message_of(
        "pub Resource() {\n    pub into: Int32 id = 1;\n    pub override fun destroy(): Void { println(\"released\"); }\n}\nnow into worker = thread { now into scratch = Resource(); };\n",
    );
    let body = message
        .split("NATIVE_HANDLER_BEGIN")
        .nth(1)
        .and_then(|rest| rest.split("NATIVE_HANDLER_END").next())
        .unwrap_or_default();
    assert!(body.contains("released"), "{message}");
}
