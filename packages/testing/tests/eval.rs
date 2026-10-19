use testing::{Package, eval, eval_with};
use types::MessageTag;

#[test]
fn evaluates_source_to_a_message() {
    let result = eval(
        r#"
init {
    into answer: Int32 = 42;
}
"#,
    )
    .unwrap();
    assert!(result.contains("NATIVE_ASSIGN\tanswer\tInt32\tn:42"));
    let tags = result
        .records()
        .iter()
        .map(|record| record.tag())
        .collect::<Vec<_>>();
    assert_eq!(
        tags,
        vec![
            MessageTag::Class,
            MessageTag::Class,
            MessageTag::Ownership,
            MessageTag::Assign,
            MessageTag::Ownership,
            MessageTag::Ownership,
        ]
    );
}

#[test]
fn provides_the_builtin_console_functions() {
    let result = eval(
        r#"
import "intolang://builtin/std";

init {
    println("hi");
}
"#,
    )
    .unwrap();
    assert_eq!(
        result.text(),
        r#"hi
"#
    );
}

#[test]
fn surfaces_lowering_errors() {
    let error = eval(
        r#"
init {
    into value: Int32 = "x";
}
"#,
    )
    .unwrap_err();
    assert!(error.to_string().contains("Type mismatch"));
}

#[test]
fn evaluates_with_extra_packages() {
    let package = Package::new("tools")
        .file(
            "src/main/into-info.into",
            r#"export "tools://math";
"#,
        )
        .file(
            "src/main/math.into",
            r#"#[link("c", "src/ffi/math.c", "math_abs")]
pub c fun abs(value: Int32): Int32;
"#,
        );
    let result = eval_with(
        r#"
import "tools://math";

init {
    into value: Int32 = abs(-4);
}
"#,
        &[package],
    )
    .unwrap();
    assert!(
        result.contains("\tmath_abs\tInt32\tvalue\ti:-4\n"),
        "{result}"
    );
}
