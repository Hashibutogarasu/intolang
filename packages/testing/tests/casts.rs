use testing::eval_vm;

const STD: &str = "import \"intolang://builtin/std\";\n";

fn output_of(body: &str) -> String {
    eval_vm(&format!("{STD}init {{\n{body}}}\n"))
        .unwrap()
        .stdout
}

fn error_of(body: &str) -> String {
    format!(
        "{:#}",
        eval_vm(&format!("{STD}init {{\n{body}}}\n")).unwrap_err()
    )
}

#[test]
fn a_character_prints_as_text() {
    let output = output_of("    into c: Char = 'A';\n    println(c);\n    println('z');\n");
    assert_eq!(output, "A\nz\n");
}

#[test]
fn a_character_converts_to_its_code() {
    let output =
        output_of("    into c: Char = 'A';\n    into n: Int32 = c as Int32;\n    println(n);\n");
    assert_eq!(output, "65\n");
}

#[test]
fn a_code_converts_to_a_character() {
    let output =
        output_of("    into n: Int32 = 97;\n    into c: Char = n as Char;\n    println(c);\n");
    assert_eq!(output, "a\n");
}

#[test]
fn a_digit_is_built_from_the_code_of_zero() {
    let output = output_of(
        "    into digit: Int32 = 7;\n    into c: Char = (('0' as Int32) + digit) as Char;\n    println(c);\n",
    );
    assert_eq!(output, "7\n");
}

#[test]
fn numbers_convert_across_widths_and_kinds() {
    let output = output_of(
        "    into big: Int32 = 300;\n    into small: Int8 = big as Int8;\n    println(small);\n    into f: Float64 = big as Float64;\n    into back: Int32 = f as Int32;\n    println(back);\n    into pi: Float64 = 3.75;\n    into whole: Int32 = pi as Int32;\n    println(whole);\n",
    );
    assert_eq!(output, "44\n300\n3\n");
}

#[test]
fn characters_compare_with_characters() {
    let output = output_of(
        "    into c: Char = 'b';\n    if (c == 'b') {\n        println(\"same\");\n    }\n    if (c < 'a') {\n        println(\"never\");\n    } else {\n        println(\"later\");\n    }\n",
    );
    assert_eq!(output, "same\nlater\n");
}

#[test]
fn a_character_cannot_take_part_in_arithmetic() {
    let message = error_of("    into c: Char = 'A';\n    into d: Char = c + c;\n");
    assert!(message.contains("numeric"), "{message}");
}

#[test]
fn a_boolean_converts_to_a_number_but_not_back() {
    let output = output_of(
        "    into yes: Boolean = 1 < 2;\n    into n: Int32 = yes as Int32;\n    println(n);\n",
    );
    assert_eq!(output, "1\n");
    let message = error_of("    into n: Int32 = 1;\n    into b: Boolean = n as Boolean;\n");
    assert!(
        message.contains("no cast from Int32 to Boolean"),
        "{message}"
    );
}

#[test]
fn a_string_does_not_convert_to_a_character() {
    let message = error_of("    into s: String = \"A\";\n    into c: Char = s as Char;\n");
    assert!(message.contains("no cast from"), "{message}");
}
