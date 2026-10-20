use ast::Program;
use types::Language;

fn program(source: &str) -> Program {
    source.parse().unwrap()
}

#[test]
fn parses_imports_exports_and_enums() {
    let program = program(
        r#"import "intolang://builtin/std";
export "json://value";
enum Format { Text, JSON = 2, }
"#,
    );
    assert_eq!(
        program.imports().next().unwrap().uri,
        "intolang://builtin/std"
    );
    assert_eq!(program.exports().next().unwrap().uri, "json://value");
    let format = program.enums().next().unwrap();
    assert_eq!(format.name, "Format");
    assert_eq!(format.variants.len(), 2);
    assert_eq!(format.variants[1].value, Some(2));
}

#[test]
fn parses_classes_with_fields_and_methods() {
    let program = program(
        r#"pub Config() {
    pub into: Int32 instance = 1;
    pub static into: Int32 count = 5;
    #[link("c", "src/a.c", "sym")]
    pub static c fun make(base: Path, n: Int32 = 2, tag: String?): Config;
    pub fun present() { this.show(); }
}
"#,
    );
    let class = program.classes().next().unwrap();
    assert!(class.is_public);
    assert_eq!(class.fields.len(), 2);
    assert!(class.fields[1].is_static);
    assert_eq!(class.methods.len(), 2);
    let make = &class.methods[0];
    assert!(make.is_public && make.is_static);
    assert_eq!(make.language, Some(Language::C));
    assert_eq!(make.attributes[0].name, "link");
    assert_eq!(
        make.attributes[0].string_args().unwrap(),
        vec!["c", "src/a.c", "sym"]
    );
    assert_eq!(make.params.len(), 3);
    assert!(make.params[2].optional);
    assert!(make.params[1].default.is_some());
    assert_eq!(make.return_type.as_ref().unwrap().as_str(), "Config");
    assert!(make.body.is_none());
    assert!(class.methods[1].body.is_some());
}

#[test]
fn parses_empty_classes_and_macros() {
    let program = program(
        r#"pub FileIterator() { }
pub macro test {
    #[test]
}
"#,
    );
    assert_eq!(program.classes().next().unwrap().name, "FileIterator");
    let macro_decl = program.macros().next().unwrap();
    assert_eq!(macro_decl.name, "test");
    assert_eq!(macro_decl.attributes[0].name, "test");
}

#[test]
fn parses_generic_and_variadic_functions() {
    let program = program(
        r#"pub fun identity<T>(value: T): T { return value; }
pub fun print(str: String, params: Object...) {}
"#,
    );
    let mut functions = program.functions();
    let identity = functions.next().unwrap();
    assert_eq!(identity.type_params, vec!["T"]);
    assert_eq!(identity.body.as_ref().unwrap().len(), 1);
    let print = functions.next().unwrap();
    assert!(print.params[1].variadic);
}

#[test]
fn rejects_legacy_class_fields() {
    let error = r#"pub Old() {
    name: String;
}
"#
    .parse::<Program>()
    .unwrap_err();
    let context = error.resolved_context();
    assert_eq!(context.span.unwrap().line, 2);
    assert!(context.message.contains("pub into: <Type> <name>"));
}

#[test]
fn rejects_dangling_attributes_and_bad_enums() {
    assert!(
        r#"#[test]
init { }"#
            .parse::<Program>()
            .is_err()
    );
    let error = "enum E { A = x }".parse::<Program>().unwrap_err();
    assert!(error.to_string().contains("32-bit integers"));
}
