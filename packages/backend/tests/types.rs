use testing::eval;

#[test]
fn lowers_explicit_integer_type() {
    let output = eval(
        r#"
init {
    into value: Int32 = 42;
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_ASSIGN\tvalue\tInt32\tn:42"),
        "{output}"
    );
}

#[test]
fn lowers_int64_and_float_types() {
    let output = eval(
        r#"
init {
    into big: Int64 = 5000000000;
    into ratio: Float = 2.5;
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_ASSIGN\tbig\tInt64\tn:5000000000"),
        "{output}"
    );
    assert!(
        output.contains("NATIVE_ASSIGN\tratio\tFloat64\tn:2.5"),
        "{output}"
    );
}

#[test]
fn accepts_numeric_widening() {
    assert!(
        eval(
            r#"
init {
    into small: Int32 = 1;
    into big: Int64 = small;
}
"#
        )
        .is_ok()
    );
}

#[test]
fn accepts_string_literal_as_path() {
    assert!(
        eval(
            r#"
init {
    into source: Path = "res/badapple.mp4";
}
"#
        )
        .is_ok()
    );
}

#[test]
fn rejects_string_assigned_to_integer() {
    let error = eval(
        r#"
init {
    into message: Int32 = "Hello World!";
}
"#,
    )
    .unwrap_err();
    assert!(error.to_string().contains("Type mismatch"), "{error}");
}

#[test]
fn rejects_mismatched_declarations() {
    for source in [
        r#"
init {
    into message: String = 42;
}
"#,
        r#"
init {
    into flag: Int32 = true;
}
"#,
        r#"
init {
    into text: String = "x";
    into count: Int32 = text;
}
"#,
        r#"
init {
    into source: Path = 42;
}
"#,
    ] {
        assert!(eval(source).is_err(), "{source}");
    }
}

#[test]
fn treats_path_as_distinct_from_string() {
    assert!(
        eval(
            r#"
init {
    into source: Path = "res/badapple.mp4";
    into text: String = source;
}
"#
        )
        .is_err()
    );
}

#[test]
fn rejects_mismatched_reassignment() {
    assert!(
        eval(
            r#"
init {
    into value: Int32 = 0;
    value = "text";
}
"#
        )
        .is_err()
    );
}
