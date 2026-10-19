use ast::Program;

fn program(source: &str) -> Program {
    source.parse().unwrap()
}

#[test]
fn a_class_lists_its_type_parameters() {
    let program = program(
        r#"pub Box<T>() {
    pub into: T content;
}
"#,
    );
    let class = program.classes().next().unwrap();
    assert_eq!(class.name, "Box");
    assert_eq!(class.type_params, vec!["T"]);
    assert_eq!(class.fields[0].ty.as_str(), "T");
}

#[test]
fn a_class_may_take_several_type_parameters() {
    let program = program(
        r#"pub Pair<K, V>() {
    pub into: K key;
    pub into: V value;
}
"#,
    );
    let class = program.classes().next().unwrap();
    assert_eq!(class.type_params, vec!["K", "V"]);
}

#[test]
fn a_parent_keeps_its_type_arguments() {
    let program = program(
        r#"pub Box<T>() {
    pub into: T content;
}
pub Named<T>(): Box<T> {
    pub into: String label = "";
}
"#,
    );
    let named = program.classes().nth(1).unwrap();
    assert_eq!(named.type_params, vec!["T"]);
    assert_eq!(named.parent.as_deref(), Some("Box"));
    assert_eq!(named.parent_type.as_ref().unwrap().as_str(), "Box<T>");
}

#[test]
fn a_parent_may_be_given_concrete_arguments() {
    let program = program(
        r#"pub Names(): Box<String> {
}
"#,
    );
    let class = program.classes().next().unwrap();
    assert!(class.type_params.is_empty());
    assert_eq!(class.parent.as_deref(), Some("Box"));
    assert_eq!(class.parent_type.as_ref().unwrap().as_str(), "Box<String>");
}

#[test]
fn a_class_without_type_parameters_has_none() {
    let program = program(
        r#"pub Plain() {
    pub into: Int32 count = 1;
}
"#,
    );
    let class = program.classes().next().unwrap();
    assert!(class.type_params.is_empty());
    assert!(class.parent_type.is_none());
}

#[test]
fn a_call_is_not_mistaken_for_a_class_header() {
    let program = program(
        r#"init {
    into b = Box();
}
"#,
    );
    assert_eq!(program.classes().count(), 0);
}
