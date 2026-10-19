#![cfg(feature = "run")]

use intovm::VmOptions;
use testing::eval_vm_bundled;

const STD: &str = "import \"intolang://builtin/std\";\n";

const CLASSES: &str = "pub Record() {
    pub into: Int32 count = 7;
    pub into: Float64 ratio = 0.5;
    pub into: String label = \"none\";
    pub into: Boolean ready = false;
    pub into: Char mark = 'a';

    pub fun reset() {
        this.count = 0;
    }
}
";

fn run(body: &str) -> Result<String, String> {
    eval_vm_bundled(
        &format!("{STD}{CLASSES}init {{\n{body}}}\n"),
        VmOptions::default(),
    )
    .map(|outcome| outcome.stdout)
    .map_err(|error| format!("{error:#}"))
}

#[test]
fn a_number_field_takes_a_new_value() {
    let output = run(
        "    into r = Record();\n    println(r.count);\n    r.count = 9;\n    println(r.count);\n",
    )
    .unwrap();
    assert_eq!(output, "7\n9\n");
}

#[test]
fn a_field_takes_the_value_of_a_variable() {
    let output = run(
        "    into r = Record();\n    into next: Int32 = 41;\n    r.count = next;\n    println(r.count);\n",
    )
    .unwrap();
    assert_eq!(output, "41\n");
}

#[test]
fn a_field_converts_a_narrower_number() {
    let output = run(
        "    into r = Record();\n    into small: Int32 = 2;\n    r.ratio = small;\n    println(r.ratio);\n    r.ratio = 1.5;\n    println(r.ratio);\n",
    )
    .unwrap();
    assert_eq!(output, "2.000000\n1.500000\n");
}

#[test]
fn a_text_field_takes_known_and_run_time_text() {
    let output = run(
        "    into r = Record();\n    println(r.label);\n    r.label = \"fixed\";\n    println(r.label);\n    into word: String = \"abc\";\n    into up = word.toUpper();\n    r.label = up + \"!\";\n    println(r.label);\n",
    )
    .unwrap();
    assert_eq!(output, "none\nfixed\nABC!\n");
}

#[test]
fn boolean_and_char_fields_take_values() {
    let output = run(
        "    into r = Record();\n    println(r.ready);\n    r.ready = true;\n    println(r.ready);\n    println(r.mark);\n    r.mark = 'z';\n    println(r.mark);\n",
    )
    .unwrap();
    assert_eq!(output, "0\n1\na\nz\n");
}

#[test]
fn a_method_assigns_the_field_of_its_receiver() {
    let output = run("    into r = Record();\n    r.reset();\n    println(r.count);\n").unwrap();
    assert_eq!(output, "0\n");
}

#[test]
fn an_unknown_field_or_name_is_an_error() {
    let message = run("    into r = Record();\n    r.missing = 1;\n").unwrap_err();
    assert!(
        message.contains("`Record` has no field `missing`"),
        "{message}"
    );
    let message = run("    nothing = 1;\n").unwrap_err();
    assert!(message.contains("Undefined name `nothing`"), "{message}");
}

#[test]
fn a_value_of_the_wrong_type_is_an_error() {
    let message = run("    into r = Record();\n    r.count = \"text\";\n").unwrap_err();
    assert!(
        message.contains("The field `count` is a `Int32`"),
        "{message}"
    );
    let message = run("    into r = Record();\n    r.label = 1;\n").unwrap_err();
    assert!(
        message.contains("The field `label` is a `String`"),
        "{message}"
    );
    let message = run("    into r = Record();\n    r.ratio = 'a';\n").unwrap_err();
    assert!(
        message.contains("The field `ratio` is a `Float64`"),
        "{message}"
    );
}
