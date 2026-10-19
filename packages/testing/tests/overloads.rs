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
fn functions_with_other_parameter_types_are_other_functions() {
    let output = run("pub fun describe(value: Int32) {
    println(\"int\");
}

pub fun describe(value: String) {
    println(\"string\");
}

pub fun describe(value: Float64) {
    println(\"float\");
}

init {
    describe(1);
    describe(\"a\");
    describe(1.5);
}
")
    .unwrap();
    assert_eq!(output, "int\nstring\nfloat\n");
}

#[test]
fn the_identical_type_wins_over_a_conversion() {
    let output = run("pub fun size(value: Int32) {
    println(\"int32\");
}

pub fun size(value: Int64) {
    println(\"int64\");
}

init {
    into small: Int32 = 1;
    into big: Int64 = 2;
    size(small);
    size(big);
}
")
    .unwrap();
    assert_eq!(output, "int32\nint64\n");
}

#[test]
fn the_number_of_arguments_decides_too() {
    let output = run("pub Adder() {
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
")
    .unwrap();
    assert_eq!(output, "4\n9\n");
}

#[test]
fn the_return_type_follows_the_chosen_function() {
    let output = run("pub fun pick(value: Int32): Int32 {
    return value + 1;
}

pub fun pick(value: String): String {
    return value + \"!\";
}

init {
    into number = pick(3);
    into text = pick(\"hey\");
    println(number);
    println(text);
    println(number + 1);
    println(text + \"?\");
}
")
    .unwrap();
    assert_eq!(output, "4\nhey!\n5\nhey!?\n");
}

#[test]
fn a_call_that_no_overload_accepts_is_an_error() {
    let message = run("pub fun describe(value: Int32) {
    println(\"int\");
}

pub fun describe(value: String) {
    println(\"string\");
}

init {
    describe(true);
}
")
    .unwrap_err();
    assert!(message.contains("describe"), "{message}");
}

#[test]
fn a_function_without_a_twin_keeps_its_name() {
    let output = run("pub fun once(value: Int32) {
    println(value);
}

init {
    once(3);
}
")
    .unwrap();
    assert_eq!(output, "3\n");
}
