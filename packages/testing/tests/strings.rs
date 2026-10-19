#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

const STD: &str = "import \"intolang://builtin/std\";\n";

fn output_of(body: &str) -> String {
    eval_vm_bundled(&format!("{STD}init {{\n{body}}}\n"), VmOptions::default())
        .unwrap()
        .stdout
}

fn error_of(body: &str) -> String {
    format!(
        "{:#}",
        eval_vm_bundled(&format!("{STD}init {{\n{body}}}\n"), VmOptions::default()).unwrap_err()
    )
}

#[test]
fn text_known_while_lowering_is_joined_while_lowering() {
    let output = output_of(
        "    into a: String = \"foo\";\n    into b: String = a + \"bar\" + '!';\n    println(b);\n",
    );
    assert_eq!(output, "foobar!\n");
}

#[test]
fn text_made_at_run_time_can_be_joined_and_printed() {
    let output = output_of(
        "    into s: String = \"abc\";\n    into up = s.toUpper();\n    into joined = up + \"-\" + s;\n    println(joined);\n",
    );
    assert_eq!(output, "ABC-abc\n");
}

#[test]
fn a_character_joins_a_run_time_text() {
    let output = output_of(
        "    into s: String = \"abc\";\n    into up = s.toUpper();\n    into c: Char = 'x';\n    into joined = up + c;\n    println(joined);\n    into front = c + up;\n    println(front);\n",
    );
    assert_eq!(output, "ABCx\nxABC\n");
}

#[test]
fn a_computed_character_joins_a_text() {
    let output = output_of(
        "    into s: String = \"n\";\n    into up = s.toUpper();\n    into digit: Int32 = 5;\n    into joined = up + (('0' as Int32) + digit) as Char;\n    println(joined);\n",
    );
    assert_eq!(output, "N5\n");
}

#[test]
fn the_text_methods_answer_for_the_text() {
    let output = output_of(
        "    into s: String = \"Hello, World\";\n    println(s.length());\n    println(s.at(1));\n    println(s.subString(7, 12));\n    println(s.indexOf(\"World\"));\n    println(s.indexOf(\"nope\"));\n    println(s.toLower());\n    println(s.toUpper());\n",
    );
    assert_eq!(output, "12\ne\nWorld\n7\n-1\nhello, world\nHELLO, WORLD\n");
}

#[test]
fn prefixes_and_suffixes_are_conditions() {
    let output = output_of(
        "    into s: String = \"Hello, World\";\n    if (s.startsWith(\"Hello\")) {\n        println(\"starts\");\n    }\n    if (s.endsWith(\"nope\")) {\n        println(\"never\");\n    } else {\n        println(\"does not end\");\n    }\n",
    );
    assert_eq!(output, "starts\ndoes not end\n");
}

#[test]
fn texts_compare_for_equality() {
    let output = output_of(
        "    into s: String = \"abc\";\n    into up = s.toUpper();\n    if (up == \"ABC\") {\n        println(\"same\");\n    }\n    if (up != s) {\n        println(\"different\");\n    }\n    into again = s.toUpper();\n    if (up == again) {\n        println(\"equal\");\n    }\n    if (s == \"abc\") {\n        println(\"constant\");\n    }\n    if (s != \"abc\") {\n        println(\"never\");\n    }\n",
    );
    assert_eq!(output, "same\ndifferent\nequal\nconstant\n");
}

#[test]
fn a_comparison_result_can_be_stored() {
    let output = output_of(
        "    into s: String = \"abc\";\n    into up = s.toUpper();\n    into same: Boolean = up == \"ABC\";\n    if (same) {\n        println(\"stored\");\n    }\n",
    );
    assert_eq!(output, "stored\n");
}

#[test]
fn a_loop_can_build_a_text() {
    let output = output_of(
        "    into s: String = \"a\";\n    into text = s.toUpper();\n    into n: Int32 = 0;\n    while (n < 3) {\n        text = text + s;\n        n = n + 1;\n    }\n    println(text);\n",
    );
    assert_eq!(output, "Aaaa\n");
}

#[test]
fn a_text_changed_by_a_loop_or_a_branch_keeps_the_value_of_the_run() {
    let output = output_of(
        "    into text: String = \"\";\n    into n: Int32 = 0;\n    while (n < 3) {\n        text = 'x' + text;\n        n = n + 1;\n    }\n    println(text);\n    into label: String = \"a\";\n    if (n == 3) {\n        label = \"b\";\n    }\n    println(label);\n    if (n == 4) {\n        label = \"c\";\n    }\n    println(label);\n",
    );
    assert_eq!(output, "xxx\nb\nb\n");
}

#[test]
fn texts_only_compare_with_equality() {
    let message = error_of(
        "    into s: String = \"abc\";\n    into up = s.toUpper();\n    if (up < \"b\") {\n        println(\"never\");\n    }\n",
    );
    assert!(message.contains("`==` and `!=`"), "{message}");
}

#[test]
fn a_text_and_a_number_do_not_join() {
    let message =
        error_of("    into s: String = \"abc\";\n    into n: Int32 = 1;\n    into t = s + n;\n");
    assert!(message.contains("Arithmetic"), "{message}");
}
