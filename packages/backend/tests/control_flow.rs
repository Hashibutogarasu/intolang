use testing::eval;

#[test]
fn lowers_numeric_while_loop() {
    let source = "init {\n    into index: Int32 = 0;\n    while (index < 10) {\n        index = index + 1;\n    }\n}\n";
    let output = eval(source).unwrap();
    assert!(
        output.contains("NATIVE_WHILE_BEGIN\tindex\t<\tn:10\tInt32"),
        "{output}"
    );
    assert!(
        output.contains("NATIVE_ARITH\tindex\tindex\tadd\tn:1\tInt32"),
        "{output}"
    );
    assert!(output.contains("NATIVE_LOOP_END"), "{output}");
}

#[test]
fn rejects_type_mismatched_arithmetic() {
    let source = "init {\n    into index: Int32 = 0;\n    into total: Int64 = index + 1;\n}\n";
    assert!(eval(source).is_err());
}

#[test]
fn lowers_if_else_with_a_variable_operand() {
    let source = "init {\n    into a: Int32 = 1;\n    into b: Int32 = 2;\n    if (a <= b) {\n        break_free();\n    } else {\n        into c: Int32 = 3;\n    }\n}\n";
    let output = eval(source).unwrap();
    let records = output.records();
    let tags = records.iter().map(|record| record.tag()).collect::<Vec<_>>();
    use types::MessageTag::*;
    assert_eq!(tags, vec![Class, Class, Assign, Assign, IfBegin, IfElse, Assign, IfEnd]);
    assert!(output.contains("NATIVE_IF_BEGIN\ta\t<=\tp:b\tInt32"), "{output}");
}

#[test]
fn break_is_only_valid_inside_a_loop() {
    let error = eval("init {\n    break;\n}\n").unwrap_err();
    assert!(error.to_string().contains("`break` must be inside a while loop"));
    let source = "init {\n    into i: Int32 = 0;\n    while (i < 3) {\n        if (i == 1) {\n            break;\n        }\n        i = i + 1;\n    }\n}\n";
    assert!(eval(source).unwrap().contains("NATIVE_BREAK"));
}

#[test]
fn rejects_non_numeric_conditions() {
    let error = eval("init {\n    into s: String = \"x\";\n    if (s == 1) {\n    }\n}\n")
        .unwrap_err();
    assert!(error.to_string().contains("if condition requires an integer or float value"));
}
