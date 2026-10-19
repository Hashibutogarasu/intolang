use testing::eval;

#[test]
fn a_thread_body_becomes_a_function_that_run_starts() {
    let message = eval(
        r#"
import "intolang://builtin/std";

now into worker = thread { println("in thread"); };
now into task = worker.run();
await task;
"#,
    )
    .unwrap()
    .to_string();
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
    let message = eval(
        r#"
import "intolang://builtin/std";

now into worker = thread { println("in thread"); };
worker.run();
"#,
    )
    .unwrap()
    .to_string();
    assert!(
        message.contains("NATIVE_THREAD_RUN\t\t__thread_"),
        "{message}"
    );
    assert!(!message.contains("NATIVE_AWAIT"), "{message}");
}

#[test]
fn awaiting_a_run_starts_and_joins_in_one_statement() {
    let message = eval(
        r#"
import "intolang://builtin/std";

now into worker = thread { println("in thread"); };
await worker.run();
"#,
    )
    .unwrap()
    .to_string();
    assert!(message.contains("NATIVE_THREAD_RUN\t__task"), "{message}");
    assert!(message.contains("NATIVE_AWAIT\t__task"), "{message}");
}

#[test]
fn threads_capture_the_values_they_mention() {
    let message = eval(
        r#"
import "intolang://builtin/std";

now into total = 1;
now into worker = thread { println(total); };
await worker.run();
"#,
    )
    .unwrap()
    .to_string();
    assert!(
        message.contains("NATIVE_HANDLER_BEGIN\t__thread_"),
        "{message}"
    );
    assert!(message.contains("\ttotal\n"), "{message}");
}

#[test]
fn await_needs_a_task() {
    let error = eval(
        r#"
import "intolang://builtin/std";

now into count = 1;
await count;
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`await` expects a Task"), "{error}");
    let error = eval(
        r#"
import "intolang://builtin/std";

now into worker = thread { println("x"); };
await worker;
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`await` expects a Task"), "{error}");
}

#[test]
fn await_is_a_statement() {
    let error = eval(
        r#"
import "intolang://builtin/std";

now into worker = thread { println("x"); };
now into value = await worker.run();
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("`await` can only be used as a statement"),
        "{error}"
    );
}

#[test]
fn locals_of_a_thread_are_destroyed_when_it_ends() {
    let message = eval(
        r#"
import "intolang://builtin/std";

pub Resource() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("released"); }
}
now into worker = thread { now into scratch = Resource(); };
"#,
    )
    .unwrap()
    .to_string();
    let body = message
        .split("NATIVE_HANDLER_BEGIN")
        .nth(1)
        .and_then(|rest| rest.split("NATIVE_HANDLER_END").next())
        .unwrap_or_default();
    assert!(body.contains("released"), "{message}");
}
