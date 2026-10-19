use testing::eval;

#[test]
fn accepts_a_concrete_subclass() {
    let output = eval(
        r#"
import "intolang://builtin/std";
pub abstract Animal() {
    pub into: String name = "x";
    pub abstract fun speak(): String;
    pub default fun describe() { println("animal"); }
    pub fun id(): Int32 { return 1; }
}
pub Dog(): Animal {
    pub override fun speak(): String { return "woof"; }
}
init {
    into dog = Dog();
}
"#,
    )
    .unwrap();
    assert!(output.contains("NATIVE_CONSTRUCT\tdog\tDog"), "{output}");
}

#[test]
fn subclasses_inherit_parent_fields_first() {
    let output = eval(
        r#"
import "intolang://builtin/std";
pub abstract Animal() {
    pub into: String name = "x";
    pub abstract fun speak(): String;
    pub default fun describe() { println("animal"); }
    pub fun id(): Int32 { return 1; }
}
pub Dog(): Animal {
    pub into: Int32 legs = 4;
    pub override fun speak(): String { return "woof"; }
}
"#,
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_CLASS\tDog\tname:String:s:78\tlegs:Int32:n:4\n"),
        "{output}"
    );
}

#[test]
fn rejects_instantiating_an_abstract_class() {
    let error = eval(
        r#"
import "intolang://builtin/std";
pub abstract Animal() {
    pub into: String name = "x";
    pub abstract fun speak(): String;
    pub default fun describe() { println("animal"); }
    pub fun id(): Int32 { return 1; }
}
init {
    into animal = Animal();
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("Cannot instantiate the abstract class `Animal`"),
        "{error}"
    );
}

#[test]
fn rejects_a_missing_override_modifier() {
    let error = eval(
        r#"
import "intolang://builtin/std";
pub abstract Animal() {
    pub into: String name = "x";
    pub abstract fun speak(): String;
    pub default fun describe() { println("animal"); }
    pub fun id(): Int32 { return 1; }
}
pub Cat(): Animal {
    pub fun speak(): String { return "meow"; }
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("hides a member of `Animal`; add `override`"),
        "{error}"
    );
}

#[test]
fn rejects_overriding_a_final_member() {
    let error = eval(
        r#"
import "intolang://builtin/std";
pub abstract Animal() {
    pub into: String name = "x";
    pub abstract fun speak(): String;
    pub default fun describe() { println("animal"); }
    pub fun id(): Int32 { return 1; }
}
pub Cat(): Animal {
    pub override fun speak(): String { return "meow"; }
    pub override fun id(): Int32 { return 2; }
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("`id` of `Animal` is final"), "{error}");
}

#[test]
fn rejects_an_override_of_nothing() {
    let error = eval(
        r#"
import "intolang://builtin/std";
pub abstract Animal() {
    pub into: String name = "x";
    pub abstract fun speak(): String;
    pub default fun describe() { println("animal"); }
    pub fun id(): Int32 { return 1; }
}
pub Cat(): Animal {
    pub override fun speak(): String { return "meow"; }
    pub override fun purr(): String { return "r"; }
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("`purr` is marked `override` but overrides nothing"),
        "{error}"
    );
}

#[test]
fn rejects_a_mismatched_override_signature() {
    let error = eval(
        r#"
import "intolang://builtin/std";
pub abstract Animal() {
    pub into: String name = "x";
    pub abstract fun speak(): String;
    pub default fun describe() { println("animal"); }
    pub fun id(): Int32 { return 1; }
}
pub Cat(): Animal {
    pub override fun speak(): Int32 { return 1; }
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("does not match the signature of `Animal.speak`"),
        "{error}"
    );
}

#[test]
fn rejects_an_unimplemented_abstract_member() {
    let error = eval(
        r#"
import "intolang://builtin/std";
pub abstract Animal() {
    pub into: String name = "x";
    pub abstract fun speak(): String;
    pub default fun describe() { println("animal"); }
    pub fun id(): Int32 { return 1; }
}
pub Cat(): Animal {
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("Class `Cat` does not implement the abstract member `speak`"),
        "{error}"
    );
}

#[test]
fn rejects_abstract_members_in_a_concrete_class() {
    let error = eval(
        r#"
pub Plain() {
    pub abstract fun run(): Void;
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("Abstract member `run` must be declared in an abstract class"),
        "{error}"
    );
}

#[test]
fn rejects_unknown_parents() {
    let error = eval(
        r#"
pub Orphan(): Missing {
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("Unknown class `Missing`"), "{error}");
}

#[test]
fn rejects_redeclaring_object_members_without_override() {
    let error = eval(
        r#"
pub Resource() {
    pub fun destroy(): Void { }
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("hides a member of `Object`; add `override`"),
        "{error}"
    );
    assert!(
        eval(
            r#"
pub Resource() {
    pub override fun destroy(): Void { }
}
"#
        )
        .is_ok()
    );
}

#[test]
fn finds_inherited_methods_on_subclass_instances() {
    let output = eval(
        r#"
import "intolang://builtin/std";
pub abstract Animal() {
    pub into: String name = "x";
    pub abstract fun speak(): String;
    pub default fun describe() { println("animal"); }
    pub fun id(): Int32 { return 1; }
}
pub Dog(): Animal {
    pub override fun speak(): String { return "woof"; }
}
init {
    into dog = Dog();
    dog.describe();
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.text(),
        r#"animal
"#
    );
}

#[test]
fn prefers_the_overriding_method() {
    let output = eval(
        r#"
import "intolang://builtin/std";
pub abstract Animal() {
    pub into: String name = "x";
    pub abstract fun speak(): String;
    pub default fun describe() { println("animal"); }
    pub fun id(): Int32 { return 1; }
}
pub Dog(): Animal {
    pub override fun speak(): String { return "woof"; }
    pub override fun describe() { println("dog"); }
}
init {
    into dog = Dog();
    dog.describe();
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.text(),
        r#"dog
"#
    );
}

#[test]
fn accepts_event_overrides_with_type_arguments() {
    assert!(
        eval(
            r#"
pub Hex() {
    pub into: String digits = "";
    pub override event onCast<Char, Hex>(from: Char, to: Hex) {}
    pub override event onCast<Int32, Hex>(from: Int32, to: Hex) {}
}
"#
        )
        .is_ok()
    );
}

#[test]
fn rejects_duplicate_event_overrides() {
    let error = eval(
        r#"
pub Hex() {
    pub override event onCast<Char, Hex>(from: Char, to: Hex) {}
    pub override event onCast<Char, Hex>(from: Char, to: Hex) {}
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("Duplicate event `onCast`"), "{error}");
}

#[test]
fn rejects_an_event_override_with_the_wrong_arity() {
    let error = eval(
        r#"
pub Hex() {
    pub override event onCast<Char>(from: Char) {}
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("does not match the signature of `Object.onCast`"),
        "{error}"
    );
}

#[test]
fn rejects_an_event_that_hides_object_without_override() {
    let error = eval(
        r#"
pub Hex() {
    pub event onCast<Char, Hex>(from: Char, to: Hex) {}
}
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("hides a member of `Object`; add `override`"),
        "{error}"
    );
}
