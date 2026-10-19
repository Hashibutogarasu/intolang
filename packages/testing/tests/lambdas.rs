#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

const STD: &str = "import \"intolang://builtin/std\";\n";

fn run(program: &str) -> Result<String, String> {
    eval_vm_bundled(&format!("{STD}{program}"), VmOptions::default())
        .map(|outcome| outcome.stdout)
        .map_err(|error| format!("{error:#}"))
}

#[test]
fn a_lambda_passed_to_a_function_is_called_by_it() {
    let output = run("pub fun each(count: Int32, action: Callback<Int32>) {
    into i: Int32 = 0;
    while (i < count) {
        action(i);
        i = i + 1;
    }
}

init {
    each(3, (n: Int32) => {
        println(n);
    });
}
")
    .unwrap();
    assert_eq!(output, "0\n1\n2\n");
}

#[test]
fn the_types_of_the_parameters_come_from_the_callback() {
    let output = run("pub fun greet(name: String, action: Callback<String>) {
    action(name);
}

init {
    greet(\"world\", (who) => {
        println(\"hello \" + who);
    });
}
")
    .unwrap();
    assert_eq!(output, "hello world\n");
}

#[test]
fn a_lambda_can_be_kept_in_a_binding() {
    let output = run("pub fun twice(action: Callback<Int32>) {
    action(1);
    action(2);
}

init {
    into show: Callback<Int32> = (n) => {
        println(n);
    };
    twice(show);
    twice(show);
}
")
    .unwrap();
    assert_eq!(output, "1\n2\n1\n2\n");
}

#[test]
fn a_lambda_sees_the_names_it_captures() {
    let output = run("pub fun each(count: Int32, action: Callback<Int32>) {
    into i: Int32 = 0;
    while (i < count) {
        action(i);
        i = i + 1;
    }
}

init {
    into total: Int32 = 0;
    each(4, (n) => {
        total = total + n;
    });
    println(total);
}
")
    .unwrap();
    assert_eq!(output, "6\n");
}

#[test]
fn a_function_with_two_parameters_uses_bi_callback() {
    let output = run("pub fun pairs(action: BiCallback<String, Int32>) {
    action(\"one\", 1);
    action(\"two\", 2);
}

init {
    pairs((name, number) => {
        println(name);
        println(number);
    });
}
")
    .unwrap();
    assert_eq!(output, "one\n1\ntwo\n2\n");
}

#[test]
fn a_function_may_create_a_lambda_that_uses_no_name_of_its_own() {
    let output = run("pub fun each(count: Int32, action: Callback<Int32>) {
    into i: Int32 = 0;
    while (i < count) {
        action(i);
        i = i + 1;
    }
}

pub fun outer() {
    each(2, (n) => {
        println(n + 10);
    });
}

init {
    outer();
}
")
    .unwrap();
    assert_eq!(output, "10\n11\n");
}

#[test]
fn a_lambda_inside_a_function_cannot_use_the_names_of_that_function() {
    let message = run("pub fun each(count: Int32, action: Callback<Int32>) {
    into i: Int32 = 0;
    while (i < count) {
        action(i);
        i = i + 1;
    }
}

pub fun outer(action: Callback<Int32>) {
    each(2, (n) => {
        action(n + 10);
    });
}

init {
    outer((value) => {
        println(value);
    });
}
")
    .unwrap_err();
    assert!(
        message.contains("cannot use the names of that function: `action`"),
        "{message}"
    );
}

#[test]
fn the_lambda_must_match_the_callback() {
    let message = run("pub fun each(action: Callback<Int32>) {
    action(1);
}

init {
    each((a, b) => {
        println(a);
    });
}
")
    .unwrap_err();
    assert!(
        message.contains("takes 1 parameter(s) but the lambda declares 2"),
        "{message}"
    );
    let message = run("pub fun each(action: Callback<Int32>) {
    action(1);
}

init {
    each((a: String) => {
        println(a);
    });
}
")
    .unwrap_err();
    assert!(
        message.contains("The parameter `a` is a `String`"),
        "{message}"
    );
}

#[test]
fn a_lambda_needs_a_callback_type() {
    let message =
        run("init {\n    into f = (n) => {\n        println(n);\n    };\n}\n").unwrap_err();
    assert!(
        message.contains("A lambda needs a callback type"),
        "{message}"
    );
}

#[test]
fn a_callback_is_called_with_the_arguments_it_declares() {
    let message = run("pub fun each(action: Callback<Int32>) {
    action(1, 2);
}

init {
    each((n) => {
        println(n);
    });
}
")
    .unwrap_err();
    assert!(
        message.contains("takes 1 argument(s) but received 2"),
        "{message}"
    );
}
