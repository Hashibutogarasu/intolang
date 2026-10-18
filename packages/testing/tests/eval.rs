use testing::{Package, eval, eval_with};
use types::MessageTag;

#[test]
fn evaluates_source_to_a_message() {
    let result = eval("init {\n    into answer: Int32 = 42;\n}\n").unwrap();
    assert!(result.contains("NATIVE_ASSIGN\tanswer\tInt32\tn:42"));
    let tags = result
        .records()
        .iter()
        .map(|record| record.tag())
        .collect::<Vec<_>>();
    assert_eq!(
        tags,
        vec![MessageTag::Class, MessageTag::Class, MessageTag::Assign]
    );
}

#[test]
fn provides_the_builtin_console_functions() {
    let result =
        eval("import \"intolang://builtin/std\";\ninit {\n    println(\"hi\");\n}\n").unwrap();
    assert_eq!(result.text(), "hi\n");
}

#[test]
fn surfaces_lowering_errors() {
    let error = eval("init {\n    into value: Int32 = \"x\";\n}\n").unwrap_err();
    assert!(error.to_string().contains("Type mismatch"));
}

#[test]
fn evaluates_with_extra_packages() {
    let package = Package::new("tools")
        .file("src/main/into-info.into", "export \"tools://math\";\n")
        .file(
            "src/main/math.into",
            "#[link(\"c\", \"src/ffi/math.c\", \"math_abs\")]\npub c fun abs(value: Int32): Int32;\n",
        );
    let result = eval_with(
        "import \"tools://math\";\ninit {\n    into value: Int32 = abs(-4);\n}\n",
        &[package],
    )
    .unwrap();
    assert!(
        result.contains("\tmath_abs\tInt32\tvalue\ti:-4\n"),
        "{result}"
    );
}
