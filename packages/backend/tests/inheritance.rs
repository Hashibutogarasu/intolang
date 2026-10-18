use testing::eval;

const ANIMALS: &str = "import \"intolang://builtin/std\";\npub abstract Animal() {\n    pub into: String name = \"x\";\n    pub abstract fun speak(): String;\n    pub default fun describe() { println(\"animal\"); }\n    pub fun id(): Int32 { return 1; }\n}\n";

fn with(extra: &str) -> String {
    format!("{ANIMALS}{extra}")
}

fn error_of(source: &str) -> String {
    eval(source).unwrap_err().to_string()
}

#[test]
fn accepts_a_concrete_subclass() {
    let source = with(
        "pub Dog(): Animal {\n    pub override fun speak(): String { return \"woof\"; }\n}\ninit {\n    into dog = Dog();\n}\n",
    );
    let output = eval(&source).unwrap();
    assert!(output.contains("NATIVE_CONSTRUCT\tdog\tDog"), "{output}");
}

#[test]
fn subclasses_inherit_parent_fields_first() {
    let source = with(
        "pub Dog(): Animal {\n    pub into: Int32 legs = 4;\n    pub override fun speak(): String { return \"woof\"; }\n}\n",
    );
    let output = eval(&source).unwrap();
    assert!(
        output.contains("NATIVE_CLASS\tDog\tname:String:s:78\tlegs:Int32:n:4\n"),
        "{output}"
    );
}

#[test]
fn rejects_instantiating_an_abstract_class() {
    let error = error_of(&with("init {\n    into animal = Animal();\n}\n"));
    assert!(
        error.contains("Cannot instantiate the abstract class `Animal`"),
        "{error}"
    );
}

#[test]
fn rejects_a_missing_override_modifier() {
    let error = error_of(&with(
        "pub Cat(): Animal {\n    pub fun speak(): String { return \"meow\"; }\n}\n",
    ));
    assert!(
        error.contains("hides a member of `Animal`; add `override`"),
        "{error}"
    );
}

#[test]
fn rejects_overriding_a_final_member() {
    let error = error_of(&with(
        "pub Cat(): Animal {\n    pub override fun speak(): String { return \"meow\"; }\n    pub override fun id(): Int32 { return 2; }\n}\n",
    ));
    assert!(error.contains("`id` of `Animal` is final"), "{error}");
}

#[test]
fn rejects_an_override_of_nothing() {
    let error = error_of(&with(
        "pub Cat(): Animal {\n    pub override fun speak(): String { return \"meow\"; }\n    pub override fun purr(): String { return \"r\"; }\n}\n",
    ));
    assert!(
        error.contains("`purr` is marked `override` but overrides nothing"),
        "{error}"
    );
}

#[test]
fn rejects_a_mismatched_override_signature() {
    let error = error_of(&with(
        "pub Cat(): Animal {\n    pub override fun speak(): Int32 { return 1; }\n}\n",
    ));
    assert!(
        error.contains("does not match the signature of `Animal.speak`"),
        "{error}"
    );
}

#[test]
fn rejects_an_unimplemented_abstract_member() {
    let error = error_of(&with("pub Cat(): Animal {\n}\n"));
    assert!(
        error.contains("Class `Cat` does not implement the abstract member `speak`"),
        "{error}"
    );
}

#[test]
fn rejects_abstract_members_in_a_concrete_class() {
    let error = error_of("pub Plain() {\n    pub abstract fun run(): Void;\n}\n");
    assert!(
        error.contains("Abstract member `run` must be declared in an abstract class"),
        "{error}"
    );
}

#[test]
fn rejects_unknown_parents() {
    let error = error_of("pub Orphan(): Missing {\n}\n");
    assert!(error.contains("Unknown class `Missing`"), "{error}");
}

#[test]
fn rejects_redeclaring_object_members_without_override() {
    let error = error_of("pub Resource() {\n    pub fun destroy(): Void { }\n}\n");
    assert!(
        error.contains("hides a member of `Object`; add `override`"),
        "{error}"
    );
    let source = "pub Resource() {\n    pub override fun destroy(): Void { }\n}\n";
    assert!(eval(source).is_ok());
}

#[test]
fn finds_inherited_methods_on_subclass_instances() {
    let source = with(
        "pub Dog(): Animal {\n    pub override fun speak(): String { return \"woof\"; }\n}\ninit {\n    into dog = Dog();\n    dog.describe();\n}\n",
    );
    let output = eval(&source).unwrap();
    assert_eq!(output.text(), "animal\n");
}

#[test]
fn prefers_the_overriding_method() {
    let source = with(
        "pub Dog(): Animal {\n    pub override fun speak(): String { return \"woof\"; }\n    pub override fun describe() { println(\"dog\"); }\n}\ninit {\n    into dog = Dog();\n    dog.describe();\n}\n",
    );
    let output = eval(&source).unwrap();
    assert_eq!(output.text(), "dog\n");
}

#[test]
fn accepts_event_overrides_with_type_arguments() {
    let source = "pub Hex() {\n    pub into: String digits = \"\";\n    pub override event onCast<Char, Hex>(from: Char, to: Hex) {}\n    pub override event onCast<Int32, Hex>(from: Int32, to: Hex) {}\n}\n";
    assert!(eval(source).is_ok());
}

#[test]
fn rejects_duplicate_event_overrides() {
    let source = "pub Hex() {\n    pub override event onCast<Char, Hex>(from: Char, to: Hex) {}\n    pub override event onCast<Char, Hex>(from: Char, to: Hex) {}\n}\n";
    let error = error_of(source);
    assert!(error.contains("Duplicate event `onCast`"), "{error}");
}

#[test]
fn rejects_an_event_override_with_the_wrong_arity() {
    let source = "pub Hex() {\n    pub override event onCast<Char>(from: Char) {}\n}\n";
    let error = error_of(source);
    assert!(
        error.contains("does not match the signature of `Object.onCast`"),
        "{error}"
    );
}

#[test]
fn rejects_an_event_that_hides_object_without_override() {
    let source = "pub Hex() {\n    pub event onCast<Char, Hex>(from: Char, to: Hex) {}\n}\n";
    let error = error_of(source);
    assert!(
        error.contains("hides a member of `Object`; add `override`"),
        "{error}"
    );
}
