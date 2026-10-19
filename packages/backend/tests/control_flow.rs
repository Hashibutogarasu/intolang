use testing::eval;

#[test]
fn lowers_numeric_while_loop() {
    let output = eval(
        r#"
init {
    into index: Int32 = 0;
    while (index < 10) {
        index = index + 1;
    }
}
"#,
    )
    .unwrap();
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
    assert!(
        eval(
            r#"
init {
    into index: Int32 = 0;
    into total: Int64 = index + 1;
}
"#
        )
        .is_err()
    );
}

#[test]
fn lowers_if_else_with_a_variable_operand() {
    let output = eval(
        r#"
init {
    into a: Int32 = 1;
    into b: Int32 = 2;
    if (a <= b) {
        break_free();
    } else {
        into c: Int32 = 3;
    }
}
"#,
    )
    .unwrap();
    let records = output.records();
    let tags = records
        .iter()
        .map(|record| record.tag())
        .collect::<Vec<_>>();
    use types::MessageTag::*;
    assert_eq!(
        tags,
        vec![
            Class, Class, Ownership, Assign, Ownership, Assign, Ownership, IfBegin, IfElse, Assign,
            Ownership, IfEnd, Ownership
        ]
    );
    assert!(
        output.contains("NATIVE_IF_BEGIN\ta\t<=\tp:b\tInt32"),
        "{output}"
    );
}

#[test]
fn break_is_only_valid_inside_a_loop() {
    let error = eval(
        r#"
init {
    break;
}
"#,
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("`break` must be inside a while loop")
    );
    assert!(
        eval(
            r#"
init {
    into i: Int32 = 0;
    while (i < 3) {
        if (i == 1) {
            break;
        }
        i = i + 1;
    }
}
"#
        )
        .unwrap()
        .contains("NATIVE_BREAK")
    );
}

#[test]
fn rejects_non_numeric_conditions() {
    let error = eval(
        r#"
init {
    into s: String = "x";
    if (s == 1) {
    }
}
"#,
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("A text can only be compared with a text, not `1`")
    );
}
