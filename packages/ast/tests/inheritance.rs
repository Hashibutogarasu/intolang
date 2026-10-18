use ast::Program;

fn program(source: &str) -> Program {
    source.parse().unwrap()
}

#[test]
fn parses_abstract_classes_and_parents() {
    let program = program(
        "pub abstract Animal() {\n    pub abstract fun speak(): String;\n    pub default fun describe(): String { return \"animal\"; }\n}\npub Dog(): Animal {\n    pub override fun speak(): String { return \"woof\"; }\n}\n",
    );
    let classes: Vec<_> = program.classes().collect();
    assert_eq!(classes.len(), 2);
    let animal = classes[0];
    assert!(animal.is_abstract && animal.parent.is_none());
    assert!(animal.methods[0].is_abstract && animal.methods[0].body.is_none());
    assert!(animal.methods[1].is_default && animal.methods[1].body.is_some());
    let dog = classes[1];
    assert!(!dog.is_abstract);
    assert_eq!(dog.parent.as_deref(), Some("Animal"));
    assert!(dog.methods[0].is_override);
}

#[test]
fn parses_events_inside_classes() {
    let program = program(
        "pub Hex() {\n    pub into: String digits = \"\";\n    pub override event onCast<Char, Hex>(from: Char, to: Hex) {\n        to.digits = \"41\";\n    }\n}\n",
    );
    let hex = program.classes().next().unwrap();
    assert_eq!(hex.fields.len(), 1);
    assert_eq!(hex.events.len(), 1);
    assert!(hex.events[0].is_override);
    assert_eq!(hex.events[0].type_params, vec!["Char", "Hex"]);
}

#[test]
fn classes_without_a_parent_keep_the_plain_header() {
    let program = program("pub Config() {\n    pub static into: Int32 count = 5;\n}\n");
    let config = program.classes().next().unwrap();
    assert!(config.parent.is_none() && !config.is_abstract);
}
