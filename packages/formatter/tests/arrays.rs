use formatter::format_source;

#[test]
fn an_array_literal_keeps_its_spelling() {
    assert_eq!(
        format_source("into xs:[Int32]=[1,2,3];"),
        r#"into xs: [Int32] = [1, 2, 3];
"#
    );
}

#[test]
fn formatting_arrays_twice_changes_nothing() {
    let once = format_source(r#"init{into names:[String]=["a","b"];names.push("c");}"#);
    assert_eq!(format_source(&once), once);
}
