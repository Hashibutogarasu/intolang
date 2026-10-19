#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

const STD: &str = "import \"intolang://builtin/std\";\n";

fn run_with(declarations: &str, body: &str) -> Result<String, String> {
    eval_vm_bundled(
        &format!("{STD}{declarations}init {{\n{body}}}\n"),
        VmOptions::default(),
    )
    .map(|outcome| outcome.stdout)
    .map_err(|error| format!("{error:#}"))
}

fn run(body: &str) -> Result<String, String> {
    run_with("", body)
}

#[test]
fn an_array_literal_makes_a_list_of_numbers() {
    let output = run("    into xs: [Int32] = [10, 20, 30];\n    println(xs.length());\n    println(xs.get(0));\n    println(xs.get(2));\n")
    .unwrap();
    assert_eq!(output, "3\n10\n30\n");
}

#[test]
fn the_element_type_comes_from_the_items() {
    let output =
        run("    into xs = [1.5, 2.5];\n    println(xs.get(1));\n    into names = [\"a\", \"b\"];\n    println(names.get(1));\n")
            .unwrap();
    assert_eq!(output, "2.500000\nb\n");
}

#[test]
fn items_are_pushed_and_replaced() {
    let output = run("    into xs: [Int32] = [1];\n    xs.push(2);\n    xs.push(3);\n    xs.set(0, 9);\n    println(xs.length());\n    println(xs.get(0));\n    println(xs.get(2));\n")
    .unwrap();
    assert_eq!(output, "3\n9\n3\n");
}

#[test]
fn an_empty_list_takes_its_type_from_the_annotation() {
    let output = run("    into xs: [String] = [];\n    xs.push(\"first\");\n    xs.push(\"second\" + \"!\");\n    println(xs.length());\n    println(xs.get(1));\n")
    .unwrap();
    assert_eq!(output, "2\nsecond!\n");
}

#[test]
fn characters_and_other_integers_are_stored_as_they_are() {
    let output = run("    into cs: [Char] = ['a', 'b'];\n    println(cs.get(1));\n    into small: [Int8] = [1, 2];\n    println(small.get(0));\n    into big: [Int64] = [5000000000];\n    println(big.get(0));\n")
    .unwrap();
    assert_eq!(output, "b\n1\n5000000000\n");
}

#[test]
fn a_list_can_hold_instances() {
    let output = run_with(
        "pub Item() {\n    pub into: String name = \"\";\n}\n",
        "    into first = Item();\n    first.name = \"one\";\n    into second = Item();\n    second.name = \"two\";\n    into items: [Item] = [];\n    items.push(first);\n    items.push(second);\n    into again = items.get(1);\n    println(again.name);\n",
    )
    .unwrap();
    assert_eq!(output, "two\n");
}

#[test]
fn an_item_of_the_wrong_type_is_an_error() {
    let message = run("    into xs: [Int32] = [1];\n    xs.push(\"text\");\n").unwrap_err();
    assert!(message.contains("Expected a `Int32`"), "{message}");
    let message = run("    into xs: [String] = [];\n    xs.push(3);\n").unwrap_err();
    assert!(message.contains("Expected a `String` item"), "{message}");
}

#[test]
fn an_empty_array_without_a_type_is_an_error() {
    let message = run("    into xs = [];\n").unwrap_err();
    assert!(message.contains("An empty array needs a type"), "{message}");
}

#[test]
fn a_loop_adds_up_the_items() {
    let output = run(
        "    into xs: [Int32] = [4, 5, 6];\n    into total: Int32 = 0;\n    into i: Int32 = 0;\n    while (i < xs.length()) {\n        total = total + xs.get(i);\n        i = i + 1;\n    }\n    println(total);\n",
    )
    .unwrap();
    assert_eq!(output, "15\n");
}
