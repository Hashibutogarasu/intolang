#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

const STD: &str = "import \"intolang://builtin/std\";\n";

const BOX: &str = "pub Box<T>() {
    pub into: T content;
}

pub Named<T>(): Box<T> {
    pub into: String label = \"unnamed\";
}
";

fn run(body: &str) -> Result<String, String> {
    eval_vm_bundled(
        &format!("{STD}{BOX}init {{\n{body}}}\n"),
        VmOptions::default(),
    )
    .map(|outcome| outcome.stdout)
    .map_err(|error| format!("{error:#}"))
}

#[test]
fn a_generic_class_holds_a_value_of_its_argument() {
    let output = run(
        "    into b: Box<String> = Box();\n    b.content = \"inside\";\n    println(b.content);\n",
    )
    .unwrap();
    assert_eq!(output, "inside\n");
}

#[test]
fn a_subclass_keeps_the_fields_and_the_arguments() {
    let output = run(
        "    into n: Named<String> = Named();\n    n.content = \"value\";\n    println(n.label);\n    println(n.content);\n",
    )
    .unwrap();
    assert_eq!(output, "unnamed\nvalue\n");
}

#[test]
fn the_argument_decides_the_type_of_the_field() {
    let message = run("    into b: Box<String> = Box();\n    b.content = 5;\n").unwrap_err();
    assert!(
        message.contains("The field `content` is a `String`"),
        "{message}"
    );
}

#[test]
fn the_same_class_with_other_arguments_is_another_type() {
    let message =
        run("    into a: Box<String> = Box();\n    into b: Box<Int32> = a;\n").unwrap_err();
    assert!(message.contains("Type mismatch"), "{message}");
}
