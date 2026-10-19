use testing::eval;

const STD: &str = "import \"intolang://builtin/std\";\n";

const SETTINGS: &str = "pub event resized(width: Int32) {}\npub Settings() {\n    pub into: Int32 width = 854;\n    pub into: String title = \"Bad Apple!!\";\n}\n";

fn with(body: &str) -> String {
    format!("{STD}{SETTINGS}init {{\n    now into settings = Settings();\n{body}}}\n")
}

#[test]
fn an_integer_field_matches_an_integer_parameter() {
    let output = eval(&with("    resized.emit(settings.width);\n")).unwrap();
    assert!(
        output.contains("NATIVE_EMIT\tresized\tf:settings:Settings:0"),
        "{output}"
    );
}

#[test]
fn a_field_of_another_type_is_reported_with_its_type() {
    let error = eval(&with("    resized.emit(settings.title);\n"))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("`resized` expects `Int32` for `width` but received `String`"),
        "{error}"
    );
}

#[test]
fn an_unknown_name_is_reported_as_an_unknown_argument() {
    let error = eval(&with("    resized.emit(missing);\n"))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("Unknown event argument `missing`"),
        "{error}"
    );
}

#[test]
fn a_field_that_does_not_exist_is_reported_as_an_unknown_argument() {
    let error = eval(&with("    resized.emit(settings.height);\n"))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("Unknown event argument `settings.height`"),
        "{error}"
    );
}
