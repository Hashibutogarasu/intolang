use testing::eval_vm;

const STD: &str = "import \"intolang://builtin/std\";\n";

const MARKER: &str = "pub Marker() {
    pub into: Int32 seen = 0;

    pub override event onCast<Char, Marker>(from: Char, to: Marker) {
        println(from);
        println(\"char to marker\");
    }

    pub override event onCast<Int32, Marker>(from: Int32, to: Marker) {
        println(from);
        println(\"int to marker\");
    }
}
";

fn run(declarations: &str, body: &str) -> Result<String, String> {
    eval_vm(&format!("{STD}{declarations}init {{\n{body}}}\n"))
        .map(|result| result.stdout)
        .map_err(|error| format!("{error:#}"))
}

#[test]
fn the_handler_for_the_source_type_runs() {
    let output = run(
        MARKER,
        "    into a: Marker = 'A' as Marker;\n    into b: Marker = 255 as Marker;\n",
    )
    .unwrap();
    assert_eq!(output, "A\nchar to marker\n255\nint to marker\n");
}

#[test]
fn a_cast_can_be_used_without_an_annotation() {
    let output = run(MARKER, "    into a = 'Z' as Marker;\n").unwrap();
    assert_eq!(output, "Z\nchar to marker\n");
}

#[test]
fn a_generic_handler_catches_what_no_concrete_one_does() {
    let declarations = "pub Wide() {
    pub into: Int32 seen = 0;

    pub override event onCast<Int32, Wide>(from: Int32, to: Wide) {
        println(\"exact\");
    }

    pub override event onCast<From, To>(from: From, to: To) {
        println(\"fallback\");
    }
}
";
    let output = run(
        declarations,
        "    into a: Wide = 1 as Wide;\n    into b: Wide = 'c' as Wide;\n",
    )
    .unwrap();
    assert_eq!(output, "exact\nfallback\n");
}

#[test]
fn a_handler_of_an_ancestor_is_found() {
    let declarations =
        format!("{MARKER}pub Child(): Marker {{\n    pub into: Int32 extra = 0;\n}}\n");
    let output = run(&declarations, "    into a: Child = 'q' as Child;\n").unwrap();
    assert_eq!(output, "q\nchar to marker\n");
}

#[test]
fn a_missing_handler_is_a_compile_error() {
    let message = run(
        MARKER,
        "    into s: String = \"x\";\n    into a: Marker = s as Marker;\n",
    )
    .unwrap_err();
    assert!(
        message.contains("no cast from String to Marker"),
        "{message}"
    );
    let message = run(
        MARKER,
        "    into f: Float64 = 1.5;\n    into a: Marker = f as Marker;\n",
    )
    .unwrap_err();
    assert!(
        message.contains("no cast from Float64 to Marker"),
        "{message}"
    );
}

#[test]
fn the_built_instance_is_owned_and_destroyed_with_its_scope() {
    let output = run(
        MARKER,
        "    into a: Marker = 'A' as Marker;\n    into b: Marker = a.copy();\n    println(\"done\");\n",
    )
    .unwrap();
    assert_eq!(output, "A\nchar to marker\ndone\n");
}
