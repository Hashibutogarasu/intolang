use testing::eval_vm;

const STD: &str = "import \"intolang://builtin/std\";\n";

fn output_of(body: &str) -> String {
    eval_vm(&format!("{STD}init {{\n{body}}}\n"))
        .unwrap()
        .stdout
}

#[test]
fn arithmetic_follows_the_precedence_of_the_operators() {
    let output = output_of(
        "    into a: Int32 = 7;\n    into b: Int32 = a % 4 + 2 * 3;\n    println(b);\n    into c: Int32 = (a & 3) | 8;\n    println(c);\n    into d: Int32 = a << 2;\n    println(d);\n    into e: Int32 = d >> 1 ^ 5;\n    println(e);\n",
    );
    assert_eq!(output, "9\n11\n28\n11\n");
}

#[test]
fn a_nested_expression_is_computed_in_temporaries() {
    let output = output_of(
        "    into x: Int32 = 2;\n    into y: Int32 = 5;\n    into z: Int32 = (x + y) * (y - x) - x;\n    println(z);\n",
    );
    assert_eq!(output, "19\n");
}

#[test]
fn logical_operators_combine_comparisons() {
    let output = output_of(
        "    into a: Int32 = 7;\n    into b: Int32 = 9;\n    if (a > 3 && b < 10) {\n        println(\"both\");\n    }\n    if (a > 100 || b == 9) {\n        println(\"either\");\n    }\n    if (!(a == 7)) {\n        println(\"never\");\n    } else {\n        println(\"not\");\n    }\n",
    );
    assert_eq!(output, "both\neither\nnot\n");
}

#[test]
fn the_right_side_of_a_logical_operator_is_skipped_when_the_left_decides() {
    let output = output_of(
        "    into a: Int32 = 7;\n    into zero: Int32 = 0;\n    if (a > 100 && 10 / zero > 1) {\n        println(\"never\");\n    } else {\n        println(\"skipped\");\n    }\n    if (a < 100 || 10 / zero > 1) {\n        println(\"short\");\n    }\n",
    );
    assert_eq!(output, "skipped\nshort\n");
}

#[test]
fn a_loop_computes_its_compound_condition_again_every_time() {
    let output = output_of(
        "    into n: Int32 = 0;\n    while (n < 10 && n * n < 30) {\n        n = n + 1;\n    }\n    println(n);\n",
    );
    assert_eq!(output, "6\n");
}

#[test]
fn a_comparison_can_be_stored_in_a_boolean() {
    let output = output_of(
        "    into a: Int32 = 7;\n    into big: Boolean = a > 3;\n    into small: Boolean = a < 3 || a == 7;\n    if (big && small) {\n        println(\"yes\");\n    }\n    if (!big) {\n        println(\"no\");\n    }\n",
    );
    assert_eq!(output, "yes\n");
}

#[test]
fn a_boolean_variable_is_a_condition() {
    let output = output_of(
        "    into ready: Boolean = true;\n    if (ready) {\n        println(\"ready\");\n    }\n    if (!ready) {\n        println(\"waiting\");\n    }\n",
    );
    assert_eq!(output, "ready\n");
}

#[test]
fn bit_operators_need_integers() {
    let error = eval_vm(&format!(
        "{STD}init {{\n    into a: Float64 = 2.5;\n    into b: Float64 = a % 2.0;\n}}\n"
    ))
    .unwrap_err()
    .to_string();
    assert!(error.contains("needs integers"), "{error}");
}

#[test]
fn operands_of_different_types_are_rejected() {
    let error = eval_vm(&format!(
        "{STD}init {{\n    into a: Int32 = 1;\n    into b: Int64 = 2;\n    into c: Int32 = a + b;\n}}\n"
    ))
    .unwrap_err()
    .to_string();
    assert!(error.contains("does not match operand type"), "{error}");
}

#[test]
fn division_by_zero_is_an_error() {
    let error = eval_vm(&format!(
        "{STD}init {{\n    into a: Int32 = 5;\n    into zero: Int32 = 0;\n    into b: Int32 = a % zero;\n}}\n"
    ))
    .unwrap_err()
    .to_string();
    assert!(error.contains("Division by zero"), "{error}");
}
