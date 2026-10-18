use testing::{Package, eval_with};

fn greet(module: &str) -> Package {
    Package::new("greet")
        .file("src/main/into-info.into", "export \"greet://hello\";\n")
        .file("src/main/hello.into", module)
}

const LINKED_MODULE: &str = "#[link(\"c\", \"src/ffi/hello.c\", \"greet_hello\")]\npub fun hello(): Void;\nfun secret(): Void;\n";

#[test]
fn resolves_module_exports() {
    let output = eval_with(
        "import \"greet://hello\";\ninit {\n    hello();\n}\n",
        &[greet(LINKED_MODULE)],
    )
    .unwrap();
    assert!(
        output.contains("NATIVE_LINK_ASSIGN\tgreet\tc\tsrc/ffi/hello.c\tgreet_hello\tVoid\t\n"),
        "{output}"
    );
}

#[test]
fn hides_private_items() {
    let output = eval_with(
        "import \"greet://hello\";\ninit {\n    secret();\n}\n",
        &[greet(LINKED_MODULE)],
    )
    .unwrap();
    assert!(!output.contains("NATIVE_LINK_ASSIGN"), "{output}");
    assert!(!output.contains("secret"), "{output}");
}

#[test]
fn emits_a_module_record_for_resolved_imports() {
    let output = eval_with("import \"greet://hello\";\n", &[greet(LINKED_MODULE)]).unwrap();
    assert!(output.contains("NATIVE_MODULE\t0\t"), "{output}");
}

#[test]
fn passes_arguments_and_defaults_to_native_calls() {
    let module = "#[link(\"c\", \"src/ffi/hello.c\", \"greet\")]\npub c fun greet(name: String, times: Int32 = 2, loud: Boolean = true): Void;\n";
    let output = eval_with(
        "import \"greet://hello\";\ninit {\n    greet(\"ab\");\n}\n",
        &[greet(module)],
    )
    .unwrap();
    assert!(
        output.contains("\tgreet\tVoid\t\ts:6162\ti:2\ti:1\n"),
        "{output}"
    );
}

#[test]
fn rejects_too_many_native_arguments() {
    let module =
        "#[link(\"c\", \"src/ffi/hello.c\", \"greet\")]\npub c fun greet(name: String): Void;\n";
    let error = eval_with(
        "import \"greet://hello\";\ninit {\n    greet(\"a\", \"b\");\n}\n",
        &[greet(module)],
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("expects 1 argument(s) but received 2"),
        "{error}"
    );
}

#[test]
fn inlines_user_functions_with_substituted_arguments() {
    let module = "pub fun show(value: Int32) {\n    println(value);\n}\n";
    let output = eval_with(
        "import \"greet://hello\";\ninit {\n    into base: Int32 = 4;\n    show(base);\n}\n",
        &[greet(module)],
    )
    .unwrap();
    assert!(output.contains("NATIVE_PRINT\tbase\tInt32\t1"), "{output}");
}

#[test]
fn inlines_methods_with_the_receiver_as_this() {
    let module = "pub Greeter() {\n    #[link(\"c\", \"src/ffi/greeter.c\", \"greeter_new\")]\n    pub c fun new(): Greeter;\n\n    #[link(\"c\", \"src/ffi/greeter.c\", \"greeter_attach\")]\n    pub c fun attach(): Void;\n\n    pub fun present() {\n        this.attach();\n    }\n}\n";
    let output = eval_with(
        "import \"greet://hello\";\ninit {\n    into greeter = Greeter();\n    greeter.present();\n}\n",
        &[greet(module)],
    )
    .unwrap();
    assert!(
        output.contains("\tgreeter_new\tGreeter\tgreeter\n"),
        "{output}"
    );
    assert!(
        output.contains("\tgreeter_attach\tVoid\t\tp:greeter\n"),
        "{output}"
    );
}
