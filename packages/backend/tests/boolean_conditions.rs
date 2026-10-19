use testing::eval;

const STD: &str = "import \"intolang://builtin/std\";\n";

#[test]
fn a_boolean_variable_is_compared_with_a_literal() {
    let output = eval(&format!(
        "{STD}init {{\n    into flag: Boolean = true;\n    if (flag == false) {{\n        println(1);\n    }}\n}}\n"
    ))
    .unwrap();
    assert!(
        output.contains("NATIVE_IF_BEGIN\tflag\t==\tn:0\tBoolean"),
        "{output}"
    );
}

#[test]
fn a_literal_on_the_left_is_mirrored() {
    let output = eval(&format!(
        "{STD}init {{\n    into count: Int32 = 1;\n    if (3 > count) {{\n        println(1);\n    }}\n}}\n"
    ))
    .unwrap();
    assert!(
        output.contains("NATIVE_IF_BEGIN\tcount\t<\tn:3\tInt32"),
        "{output}"
    );
}

#[test]
fn comparisons_between_literals_are_decided_while_lowering() {
    let output = eval(&format!(
        "{STD}pub fun check(left: Int32, right: Int32) {{\n    if (left != right) {{\n        println(1);\n    }}\n}}\ninit {{\n    check(3, 3);\n    check(3, 4);\n}}\n"
    ))
    .unwrap();
    assert!(output.contains("NATIVE_IF_CONST\t0"), "{output}");
    assert!(output.contains("NATIVE_IF_CONST\t1"), "{output}");
}
