use std::path::Path;
use testing::{Package, Workspace};

const SOURCE: &str = "import \"intolang://builtin/tests/assert\";\nimport \"intolang://builtin/tests/ownership-check\";\n\npub Dog() {\n    pub into: Int32 age = 1;\n}\n\npub fun first() {\n    now into dog = Dog();\n    assertOwned(dog);\n    assertTrue(true);\n}\n\npub fun second() {\n    println(1);\n}\n";

fn helpers() -> Package {
    Package::new("builtin")
        .file(
            "src/main/into-info.into",
            "export \"builtin://tests/assert/assertTrue\";\nexport \"builtin://tests/ownership-check/assertOwned\";\n",
        )
        .file(
            "src/main/tests/assert.into",
            "pub fun assertTrue(condition: Boolean) {\n    if condition == false {\n        throw \"expected true\";\n    }\n}\n",
        )
        .file(
            "src/main/tests/ownership-check.into",
            "#[inspects]\npub fun assertOwned(value: Object) {\n    if value.isOwned == false {\n        throw \"value is not owned\";\n    }\n}\n",
        )
}

fn lower(test: &str) -> anyhow::Result<String> {
    let workspace = Workspace::with_packages(&[helpers()])?;
    backend::lower_test(
        SOURCE,
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
