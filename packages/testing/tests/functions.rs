#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

#[test]
fn a_recursive_function_calls_itself_at_run_time() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun fib(n: Int32): Int32 {
    if (n < 2) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

init {
    into r = fib(10);
    println(r);
    into small = fib(1);
    println(small);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"55
1
"#
    );
}

#[test]
fn a_function_returns_before_its_last_statement() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun sign(n: Int32): Int32 {
    if (n < 0) {
        return 0 - 1;
    }
    if (n == 0) {
        return 0;
    }
    return 1;
}

init {
    into a = sign(7);
    println(a);
    into b = sign(0);
    println(b);
    into c = sign(0 - 5);
    println(c);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"1
0
-1
"#
    );
}

#[test]
fn a_function_that_returns_nothing_can_stop_early() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun report(n: Int32) {
    if (n > 2) {
        println("big");
        return;
    }
    println("small");
}

init {
    report(1);
    report(5);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"small
big
"#
    );
}

#[test]
fn a_recursive_function_can_return_text() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun repeat(text: String, count: Int32): String {
    if (count < 1) {
        return "";
    }
    return text + repeat(text, count - 1);
}

init {
    into line = repeat("ab", 3);
    println(line);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"ababab
"#
    );
}

#[test]
fn functions_that_call_each_other_are_called_at_run_time() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun even(n: Int32): Int32 {
    if (n == 0) {
        return 1;
    }
    return odd(n - 1);
}

pub fun odd(n: Int32): Int32 {
    if (n == 0) {
        return 0;
    }
    return even(n - 1);
}

init {
    println(even(10));
    println(even(7));
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"1
0
"#
    );
}

#[test]
fn a_recursive_method_receives_its_instance() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Counter() {
    pub into: Int32 step = 3;

    pub fun total(n: Int32): Int32 {
        if (n < 1) {
            return 0;
        }
        return this.step + this.total(n - 1);
    }
}

init {
    into c = Counter();
    into sum = c.total(4);
    println(sum);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"12
"#
    );
}

#[test]
fn an_argument_is_converted_to_the_type_of_its_parameter() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun half(n: Float64): Float64 {
    if (n < 1.0) {
        return n;
    }
    return half(n / 2.0);
}

init {
    into whole: Int32 = 8;
    into r = half(whole);
    println(r);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"0.500000
"#
    );
}

#[test]
fn a_call_with_the_wrong_arguments_is_an_error() {
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

pub fun fib(n: Int32): Int32 {
    if (n < 2) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

init {
    into r = fib("ten");
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(message.contains("Expected a `Int32`"), "{message}");
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

pub fun fib(n: Int32): Int32 {
    if (n < 2) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

init {
    into r = fib(1, 2);
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(message.contains("expects 1 argument(s)"), "{message}");
}

#[test]
fn endless_recursion_overflows_the_stack_of_the_vm() {
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

pub fun forever(n: Int32): Int32 {
    return forever(n + 1);
}

init {
    into r = forever(0);
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(message.contains("Stack overflow"), "{message}");
}
