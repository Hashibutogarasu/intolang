use std::path::Path;
use testing::{Package, Workspace};

fn helpers() -> Package {
    Package::new("builtin")
        .file(
            "src/main/into-info.into",
            r#"export "builtin://tests/assert/assertTrue";
export "builtin://tests/ownership-check/assertOwned";
"#,
        )
        .file(
            "src/main/tests/assert.into",
            r#"pub fun assertTrue(condition: Boolean) {
    if condition == false {
        throw "expected true";
    }
}
"#,
        )
        .file(
            "src/main/tests/ownership-check.into",
            r#"#[inspects]
pub fun assertOwned(value: Object) {
    if value.isOwned == false {
        throw "value is not owned";
    }
}
"#,
        )
}

fn lower(test: &str) -> anyhow::Result<String> {
    let workspace = Workspace::with_packages(&[helpers()])?;
    backend::lower_test(
        r#"import "intolang://builtin/tests/assert";
import "intolang://builtin/tests/ownership-check";

pub Dog() {
    pub into: Int32 age = 1;
}

pub fun first() {
    now into dog = Dog();
    assertOwned(dog);
    assertTrue(true);
}

pub fun second() {
    println(1);
}
"#,
        Path::new("sample.into"),
        Some(workspace.path()),
        test,
    )
}

#[test]
fn only_the_named_function_is_lowered() {
    let message = lower("first").unwrap();
    assert!(
        message.contains("NATIVE_IF_STATE\tdog\tisOwned\t0"),
        "{message}"
    );
    assert!(message.contains("NATIVE_IF_CONST\t0"), "{message}");
    assert!(!message.contains("NATIVE_PRINT"), "{message}");
}

#[test]
fn an_unknown_function_name_is_an_error() {
    let error = lower("third").unwrap_err();
    assert!(error.to_string().contains("no function `third`"), "{error}");
}
