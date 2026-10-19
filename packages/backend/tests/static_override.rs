use testing::eval;

const SOURCE: &str = "import \"intolang://builtin/std\";\npub Base() {\n    pub default static fun make(): Base { return 1; }\n    pub static fun fixed(): Base { return 1; }\n}\npub Other() {\n}\n";

fn with(extra: &str) -> String {
    format!("{SOURCE}{extra}")
}

fn error_of(source: &str) -> String {
    eval(source).unwrap_err().to_string()
}

#[test]
fn a_static_method_may_be_default_and_overridden() {
    let source =
        with("pub Child(): Base {\n    pub override static fun make(): Base { return 1; }\n}\n");
    assert!(eval(&source).is_ok());
}

#[test]
fn an_override_may_return_the_overriding_class() {
    let source =
        with("pub Child(): Base {\n    pub override static fun make(): Child { return 1; }\n}\n");
    assert!(eval(&source).is_ok());
}

#[test]
fn an_override_may_not_return_an_unrelated_class() {
    let error = error_of(&with(
        "pub Child(): Base {\n    pub override static fun make(): Other { return 1; }\n}\n",
    ));
    assert!(
        error.contains("`make` does not match the signature of `Base.make`"),
        "{error}"
    );
}

#[test]
fn a_static_method_that_is_not_default_stays_final() {
    let error = error_of(&with(
        "pub Child(): Base {\n    pub override static fun fixed(): Child { return 1; }\n}\n",
    ));
    assert!(
        error.contains("`fixed` of `Base` is final and cannot be overridden"),
        "{error}"
    );
}
