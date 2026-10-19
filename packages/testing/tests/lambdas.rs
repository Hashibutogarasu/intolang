#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

#[test]
fn a_lambda_passed_to_a_function_is_called_by_it() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun each(count: Int32, action: Callback<Int32>) {
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
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"0
1
2
"#
    );
}

#[test]
fn the_types_of_the_parameters_come_from_the_callback() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun greet(name: String, action: Callback<String>) {
    action(name);
}

init {
    greet("world", (who) => {
        println("hello " + who);
    });
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"hello world
"#
    );
}

#[test]
fn a_lambda_can_be_kept_in_a_binding() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun twice(action: Callback<Int32>) {
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
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"1
2
1
2
"#
    );
}

#[test]
fn a_lambda_sees_the_names_it_captures() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun each(count: Int32, action: Callback<Int32>) {
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
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"6
"#
    );
}

#[test]
fn a_function_with_two_parameters_uses_bi_callback() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun pairs(action: BiCallback<String, Int32>) {
    action("one", 1);
    action("two", 2);
}

init {
    pairs((name, number) => {
        println(name);
        println(number);
    });
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"one
1
two
2
"#
    );
}

#[test]
fn a_function_may_create_a_lambda_that_uses_no_name_of_its_own() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun each(count: Int32, action: Callback<Int32>) {
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
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"10
11
"#
    );
}

#[test]
fn a_lambda_inside_a_function_cannot_use_the_names_of_that_function() {
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

pub fun each(count: Int32, action: Callback<Int32>) {
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
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(
        message.contains("cannot use the names of that function: `action`"),
        "{message}"
    );
}

#[test]
fn the_lambda_must_match_the_callback() {
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

pub fun each(action: Callback<Int32>) {
    action(1);
}

init {
    each((a, b) => {
        println(a);
    });
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(
        message.contains("takes 1 parameter(s) but the lambda declares 2"),
        "{message}"
    );
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

pub fun each(action: Callback<Int32>) {
    action(1);
}

init {
    each((a: String) => {
        println(a);
    });
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(
        message.contains("The parameter `a` is a `String`"),
        "{message}"
    );
}

#[test]
fn a_lambda_needs_a_callback_type() {
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

init {
    into f = (n) => {
        println(n);
    };
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(
        message.contains("A lambda needs a callback type"),
        "{message}"
    );
}

#[test]
fn a_callback_is_called_with_the_arguments_it_declares() {
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

pub fun each(action: Callback<Int32>) {
    action(1, 2);
}

init {
    each((n) => {
        println(n);
    });
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(
        message.contains("takes 1 argument(s) but received 2"),
        "{message}"
    );
}
