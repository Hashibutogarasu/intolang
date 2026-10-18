use testing::eval;

#[test]
fn excludes_static_fields_from_class_layout() {
    let source = "pub Config() {\n    pub into: Int32 instance = 1;\n    pub static into: Int32 count = 5;\n}\n";
    let output = eval(source).unwrap();
    assert!(
        output.contains("NATIVE_CLASS\tConfig\tinstance:Int32:n:1\n"),
        "{output}"
    );
}

#[test]
fn resolves_static_field_constant() {
    let source = "pub Config() {\n    pub static into: Int32 count = 5;\n}\ninit {\n    into value: Int32 = Config.count;\n}\n";
    let output = eval(source).unwrap();
    assert!(
        output.contains("NATIVE_ASSIGN\tvalue\tInt32\tn:5"),
        "{output}"
    );
}

#[test]
fn encodes_string_and_boolean_field_defaults() {
    let source = "pub Info() {\n    pub into: String name = \"ab\";\n    pub into: Boolean ready = true;\n    pub into: Int32 plain;\n}\n";
    let output = eval(source).unwrap();
    assert!(
        output.contains(
            "NATIVE_CLASS\tInfo\tname:String:s:6162\tready:Boolean:b:1\tplain:Int32:-:\n"
        ),
        "{output}"
    );
}

#[test]
fn constructs_declared_classes_in_declarations() {
    let source = "pub Config() {\n    pub into: Int32 instance = 1;\n}\ninit {\n    into config = Config();\n}\n";
    let output = eval(source).unwrap();
    assert!(
        output.contains("NATIVE_CONSTRUCT\tconfig\tConfig"),
        "{output}"
    );
}

#[test]
fn prints_instance_fields() {
    let source = "import \"intolang://builtin/std\";\npub Config() {\n    pub into: Int32 instance = 1;\n}\ninit {\n    into config = Config();\n    println(config.instance);\n}\n";
    let output = eval(source).unwrap();
    assert!(
        output.contains("NATIVE_FIELD_PRINT\tconfig\tConfig\t0\tInt32\t1"),
        "{output}"
    );
}

#[test]
fn rejects_the_legacy_field_syntax() {
    let error = eval("pub Old() {\n    name: String;\n}\n").unwrap_err();
    assert!(
        error.to_string().contains("pub into: <Type> <name>"),
        "{error}"
    );
}
