use testing::eval;

#[test]
fn registering_and_emitting_lowers_to_handler_records() {
    let message = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
example.on((value) => { println(value); });
example.emit(5);
"#,
    )
    .unwrap()
    .to_string();
    assert!(
        message.contains("NATIVE_HANDLER_BEGIN\t__handler_example_"),
        "{message}"
    );
    assert!(message.contains("\tvalue:Int32"), "{message}");
    assert!(
        message.contains("NATIVE_EVENT_ON\texample\t\t__handler_example_"),
        "{message}"
    );
    assert!(message.contains("NATIVE_EMIT\texample\ti:5"), "{message}");
}

#[test]
fn the_lambda_may_name_its_parameters_freely() {
    let message = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
example.on((renamed) => { println(renamed); });
"#,
    )
    .unwrap()
    .to_string();
    assert!(
        message.contains("NATIVE_PRINT\trenamed\tInt32"),
        "{message}"
    );
}

#[test]
fn handlers_capture_the_values_they_mention() {
    let message = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
now into total = 5;
now into other = 6;
example.on((value) => { println(total); });
"#,
    )
    .unwrap()
    .to_string();
    assert!(message.contains("\ttotal\tvalue:Int32"), "{message}");
    assert!(!message.contains("\ttotal,other\t"), "{message}");
}

#[test]
fn the_body_of_an_event_is_its_default_handler() {
    let message = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {
    println("default");
}
"#,
    )
    .unwrap()
    .to_string();
    assert!(
        message.contains("NATIVE_HANDLER_BEGIN\t__default_example"),
        "{message}"
    );
    assert!(
        message.contains("NATIVE_EVENT_ON\texample\t\t__default_example"),
        "{message}"
    );
}

#[test]
fn on_returns_a_listener_that_unregisters_once() {
    let message = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
now into listener = example.on((value) => { println(value); });
listener.destroy();
"#,
    )
    .unwrap()
    .to_string();
    assert!(
        message.contains("NATIVE_EVENT_ON\texample\tlistener\t"),
        "{message}"
    );
    assert!(
        message.contains("NATIVE_EVENT_OFF\texample\tlistener"),
        "{message}"
    );
    let error = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
now into listener = example.on((value) => { println(value); });
listener.destroy();
listener.destroy();
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`listener` was destroyed"), "{error}");
}

#[test]
fn a_listener_is_unregistered_when_its_scope_ends() {
    let message = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
now into n = 1;
if (n == 1) {
    now into listener = example.on((value) => { println(value); });
}
"#,
    )
    .unwrap()
    .to_string();
    assert!(
        message.contains("NATIVE_EVENT_OFF\texample\tlistener"),
        "{message}"
    );
}

#[test]
fn destroying_an_event_blocks_every_later_use() {
    let message = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
example.destroy();
"#,
    )
    .unwrap()
    .to_string();
    assert!(
        message.contains("NATIVE_EVENT_DESTROY\texample"),
        "{message}"
    );
    let error = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
example.destroy();
example.emit(1);
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`example` was destroyed"), "{error}");
    let error = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
example.destroy();
example.on((value) => { println(value); });
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`example` was destroyed"), "{error}");
    let error = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
example.destroy();
example.destroy();
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`example` was destroyed"), "{error}");
}

#[test]
fn an_event_assigned_to_a_binding_is_the_same_event() {
    let message = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
now into e = example;
e.on((value) => { println(value); });
e.emit(2);
"#,
    )
    .unwrap()
    .to_string();
    assert!(
        message.contains("NATIVE_EVENT_ON\texample\t\t"),
        "{message}"
    );
    assert!(message.contains("NATIVE_EMIT\texample\ti:2"), "{message}");
    let error = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
now into e = example;
e.destroy();
example.emit(2);
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`example` was destroyed"), "{error}");
    let error = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
now into e = example;
e.destroy();
e.emit(2);
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`e` was destroyed"), "{error}");
}

#[test]
fn emit_checks_the_arguments_against_the_declaration() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
example.emit();
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("takes 1 argument"), "{error}");
    let error = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
example.emit("text");
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("expects `Int32`"), "{error}");
}

#[test]
fn the_handler_must_match_the_declared_parameters() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
example.on((a, b) => { println(a); });
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("takes 1 parameter"), "{error}");
    let error = eval(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
example.on(5);
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("expects one lambda"), "{error}");
}

#[test]
fn locals_of_a_handler_are_destroyed_when_it_ends() {
    let message = eval(
        r#"
import "intolang://builtin/std";

pub Resource() {
    pub into: Int32 id = 1;
    pub override fun destroy(): Void { println("released"); }
}
pub event example(value: Int32) {}
example.on((value) => { now into scratch = Resource(); });
"#,
    )
    .unwrap();
    let text = message.to_string();
    let handler = text
        .split("NATIVE_HANDLER_BEGIN")
        .nth(1)
        .and_then(|rest| rest.split("NATIVE_HANDLER_END").next())
        .unwrap_or_default();
    assert!(handler.contains("released"), "{text}");
}
