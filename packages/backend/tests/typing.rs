use testing::eval;

#[test]
fn an_integer_field_matches_an_integer_parameter() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub event resized(width: Int32) {}
pub Settings() {
    pub into: Int32 width = 854;
    pub into: String title = "Bad Apple!!";
}
init {
    now into settings = Settings();
    resized.emit(settings.width);
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_EMIT\tresized\tf:settings:Settings:0"),
        "{output}"
    );
}

#[test]
fn a_field_of_another_type_is_reported_with_its_type() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub event resized(width: Int32) {}
pub Settings() {
    pub into: Int32 width = 854;
    pub into: String title = "Bad Apple!!";
}
init {
    now into settings = Settings();
    resized.emit(settings.title);
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("`resized` expects `Int32` for `width` but received `String`"),
        "{error}"
    );
}

#[test]
fn an_unknown_name_is_reported_as_an_unknown_argument() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub event resized(width: Int32) {}
pub Settings() {
    pub into: Int32 width = 854;
    pub into: String title = "Bad Apple!!";
}
init {
    now into settings = Settings();
    resized.emit(missing);
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("Unknown event argument `missing`"),
        "{error}"
    );
}

#[test]
fn a_field_that_does_not_exist_is_reported_as_an_unknown_argument() {
    let error = eval(
        r#"
import "intolang://builtin/std";

pub event resized(width: Int32) {}
pub Settings() {
    pub into: Int32 width = 854;
    pub into: String title = "Bad Apple!!";
}
init {
    now into settings = Settings();
    resized.emit(settings.height);
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("Unknown event argument `settings.height`"),
        "{error}"
    );
}
