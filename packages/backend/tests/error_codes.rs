use errors::{Domain, Error, Layer};
use testing::eval;

fn failure(source: &str) -> Error {
    eval(source)
        .unwrap_err()
        .downcast::<Error>()
        .expect("the failure is an error of the toolchain")
}

fn has_into_layer(error: &Error) -> bool {
    error
        .trace()
        .layers()
        .iter()
        .any(|layer| layer.layer() == Layer::Into)
}

#[test]
fn a_syntax_error_has_the_syntax_code_and_the_path_of_the_source() {
    let error = failure("}");
    assert_eq!(error.code(), Some(0x0001));
    assert_eq!(error.domain(), Domain::IntoLang);
    assert_eq!(error.resolved_context().path.as_deref(), Some("<source>"));
    assert!(has_into_layer(&error));
}

#[test]
fn a_variable_out_of_scope_has_its_own_code() {
    let error = failure(
        r#"import "intolang://builtin/std";
into mem1 = 0;
print(mem1);
"#,
    );
    assert_eq!(error.code(), Some(0x1040));
    assert!(has_into_layer(&error));
    assert!(error.to_string().contains("module scope at <source>:3"));
}

#[test]
fn a_destroyed_binding_has_the_ownership_code() {
    let error = failure(
        r#"
import "intolang://builtin/std";

pub event example(value: Int32) {}
example.destroy();
example.emit(1);
"#,
    );
    assert_eq!(error.code(), Some(0x2003));
    assert!(error.to_string().contains("`example` was destroyed"));
}

#[test]
fn an_unterminated_string_has_its_own_code_and_a_line() {
    let error = failure("into a = \"abc;");
    assert_eq!(error.code(), Some(0x0008));
    assert!(has_into_layer(&error));
}
