use testing::eval;

#[test]
fn excludes_static_fields_from_class_layout() {
    let output = eval(
        r#"
pub Config() {
    pub into: Int32 instance = 1;
    pub static into: Int32 count = 5;
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_CLASS\tConfig\tinstance:Int32:n:1\n"),
        "{output}"
    );
}

#[test]
fn resolves_static_field_constant() {
    let output = eval(
        r#"
pub Config() {
    pub static into: Int32 count = 5;
}

init {
    into value: Int32 = Config.count;
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_ASSIGN\tvalue\tInt32\tn:5"),
        "{output}"
    );
}

#[test]
fn encodes_string_and_boolean_field_defaults() {
    let output = eval(
        r#"
pub Info() {
    pub into: String name = "ab";
    pub into: Boolean ready = true;
    pub into: Int32 plain;
}
"#,
    )
    .unwrap();
    assert!(
        output.contains(
            "NATIVE_CLASS\tInfo\tname:String:s:6162\tready:Boolean:b:1\tplain:Int32:-:\n"
        ),
        "{output}"
    );
}

#[test]
fn constructs_declared_classes_in_declarations() {
    let output = eval(
        r#"
pub Config() {
    pub into: Int32 instance = 1;
}

init {
    into config = Config();
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_CONSTRUCT\tconfig\tConfig"),
        "{output}"
    );
}

#[test]
fn prints_instance_fields() {
    let output = eval(
        r#"
import "intolang://builtin/std";

pub Config() {
    pub into: Int32 instance = 1;
}

init {
    into config = Config();
    println(config.instance);
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_FIELD_PRINT\tconfig\tConfig\t0\tInt32\t1"),
        "{output}"
    );
}

#[test]
fn rejects_the_legacy_field_syntax() {
    let error = eval(
        r#"
pub Old() {
    name: String;
}
"#,
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("pub into: <Type> <name>"),
        "{error}"
    );
}
