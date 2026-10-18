use testing::eval;

#[test]
fn lowers_explicit_integer_type() {
    let output = eval("init {\n    into value: Int32 = 42;\n}\n").unwrap();
    assert!(
        output.contains("NATIVE_ASSIGN\tvalue\tInt32\tn:42"),
        "{output}"
    );
}

#[test]
fn lowers_int64_and_float_types() {
    let output =
        eval("init {\n    into big: Int64 = 5000000000;\n    into ratio: Float = 2.5;\n}\n")
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
    let source = "init {\n    into small: Int32 = 1;\n    into big: Int64 = small;\n}\n";
    assert!(eval(source).is_ok());
}

#[test]
fn accepts_string_literal_as_path() {
    assert!(eval("init {\n    into source: Path = \"res/badapple.mp4\";\n}\n").is_ok());
}

#[test]
fn rejects_string_assigned_to_integer() {
    let error = eval("init {\n    into message: Int32 = \"Hello World!\";\n}\n").unwrap_err();
    assert!(error.to_string().contains("Type mismatch"), "{error}");
}

#[test]
fn rejects_mismatched_declarations() {
    for source in [
        "init {\n    into message: String = 42;\n}\n",
        "init {\n    into flag: Int32 = true;\n}\n",
        "init {\n    into text: String = \"x\";\n    into count: Int32 = text;\n}\n",
        "init {\n    into source: Path = 42;\n}\n",
    ] {
        assert!(eval(source).is_err(), "{source}");
    }
}

#[test]
fn treats_path_as_distinct_from_string() {
    let source = "init {\n    into source: Path = \"res/badapple.mp4\";\n    into text: String = source;\n}\n";
    assert!(eval(source).is_err());
}

#[test]
fn rejects_mismatched_reassignment() {
    let source = "init {\n    into value: Int32 = 0;\n    value = \"text\";\n}\n";
    assert!(eval(source).is_err());
}
