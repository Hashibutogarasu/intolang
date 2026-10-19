use testing::eval;

#[test]
fn a_static_method_may_be_default_and_overridden() {
    assert!(
        eval(
            r#"
import "intolang://builtin/std";
pub Base() {
    pub default static fun make(): Base { return 1; }
    pub static fun fixed(): Base { return 1; }
}
pub Other() {
}
pub Child(): Base {
    pub override static fun make(): Base { return 1; }
}
"#
        )
        .is_ok()
    );
}

#[test]
fn an_override_may_return_the_overriding_class() {
    assert!(
        eval(
            r#"
import "intolang://builtin/std";
pub Base() {
    pub default static fun make(): Base { return 1; }
    pub static fun fixed(): Base { return 1; }
}
pub Other() {
}
pub Child(): Base {
    pub override static fun make(): Child { return 1; }
}
"#
        )
        .is_ok()
    );
}

#[test]
fn an_override_may_not_return_an_unrelated_class() {
    let error = eval(
        r#"
import "intolang://builtin/std";
pub Base() {
    pub default static fun make(): Base { return 1; }
    pub static fun fixed(): Base { return 1; }
}
pub Other() {
}
pub Child(): Base {
    pub override static fun make(): Other { return 1; }
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("`make` does not match the signature of `Base.make`"),
        "{error}"
    );
}

#[test]
fn a_static_method_that_is_not_default_stays_final() {
    let error = eval(
        r#"
import "intolang://builtin/std";
pub Base() {
    pub default static fun make(): Base { return 1; }
    pub static fun fixed(): Base { return 1; }
}
pub Other() {
}
pub Child(): Base {
    pub override static fun fixed(): Child { return 1; }
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("`fixed` of `Base` is final and cannot be overridden"),
        "{error}"
    );
}
