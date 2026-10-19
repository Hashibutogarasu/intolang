#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

#[test]
fn functions_with_other_parameter_types_are_other_functions() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun describe(value: Int32) {
    println("int");
}

pub fun describe(value: String) {
    println("string");
}

pub fun describe(value: Float64) {
    println("float");
}

init {
    describe(1);
    describe("a");
    describe(1.5);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"int
string
float
"#
    );
}

#[test]
fn the_identical_type_wins_over_a_conversion() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun size(value: Int32) {
    println("int32");
}

pub fun size(value: Int64) {
    println("int64");
}

init {
    into small: Int32 = 1;
    into big: Int64 = 2;
    size(small);
    size(big);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"int32
int64
"#
    );
}

#[test]
fn the_number_of_arguments_decides_too() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub Adder() {
    pub fun add(a: Int32) {
        println(a);
    }

    pub fun add(a: Int32, b: Int32) {
        println(a + b);
    }
}

init {
    into adder = Adder();
    adder.add(4);
    adder.add(4, 5);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"4
9
"#
    );
}

#[test]
fn the_return_type_follows_the_chosen_function() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun pick(value: Int32): Int32 {
    return value + 1;
}

pub fun pick(value: String): String {
    return value + "!";
}

init {
    into number = pick(3);
    into text = pick("hey");
    println(number);
    println(text);
    println(number + 1);
    println(text + "?");
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"4
hey!
5
hey!?
"#
    );
}

#[test]
fn a_call_that_no_overload_accepts_is_an_error() {
    let message = format!(
        "{:#}",
        eval_vm_bundled(
            r#"
import "intolang://builtin/std";

pub fun describe(value: Int32) {
    println("int");
}

pub fun describe(value: String) {
    println("string");
}

init {
    describe(true);
}
"#,
            VmOptions::default(),
        )
        .unwrap_err()
    );
    assert!(message.contains("describe"), "{message}");
}

#[test]
fn a_function_without_a_twin_keeps_its_name() {
    let output = eval_vm_bundled(
        r#"
import "intolang://builtin/std";

pub fun once(value: Int32) {
    println(value);
}

init {
    once(3);
}
"#,
        VmOptions::default(),
    )
    .unwrap();
    assert_eq!(
        output.stdout,
        r#"3
"#
    );
}
