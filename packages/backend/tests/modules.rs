use testing::{Package, eval_with};

fn greet(module: &str) -> Package {
    Package::new("greet")
        .file(
            "src/main/into-info.into",
            r#"export "greet://hello";
"#,
        )
        .file("src/main/hello.into", module)
}

#[test]
fn resolves_module_exports() {
    let output = eval_with(
        r#"
import "greet://hello";

init {
    hello();
}
"#,
        &[greet(
            r#"#[link("c", "src/ffi/hello.c", "greet_hello")]
pub fun hello(): Void;
fun secret(): Void;
"#,
        )],
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
        r#"
import "greet://hello";

init {
    secret();
}
"#,
        &[greet(
            r#"#[link("c", "src/ffi/hello.c", "greet_hello")]
pub fun hello(): Void;
fun secret(): Void;
"#,
        )],
    )
    .unwrap();
    assert!(!output.contains("NATIVE_LINK_ASSIGN"), "{output}");
    assert!(!output.contains("secret"), "{output}");
}

#[test]
fn emits_a_module_record_for_resolved_imports() {
    let output = eval_with(
        r#"
import "greet://hello";
"#,
        &[greet(
            r#"#[link("c", "src/ffi/hello.c", "greet_hello")]
pub fun hello(): Void;
fun secret(): Void;
"#,
        )],
    )
    .unwrap();
    assert!(output.contains("NATIVE_MODULE\t0\t"), "{output}");
}

#[test]
fn passes_arguments_and_defaults_to_native_calls() {
    let output = eval_with(
        r#"
import "greet://hello";

init {
    greet("ab");
}
"#,
        &[greet(
            r#"#[link("c", "src/ffi/hello.c", "greet")]
pub c fun greet(name: String, times: Int32 = 2, loud: Boolean = true): Void;
"#,
        )],
    )
    .unwrap();
    assert!(
        output.contains("\tgreet\tVoid\t\ts:6162\ti:2\ti:1\n"),
        "{output}"
    );
}

#[test]
fn rejects_too_many_native_arguments() {
    let error = eval_with(
        r#"
import "greet://hello";

init {
    greet("a", "b");
}
"#,
        &[greet(
            r#"#[link("c", "src/ffi/hello.c", "greet")]
pub c fun greet(name: String): Void;
"#,
        )],
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
    let output = eval_with(
        r#"
import "greet://hello";

init {
    into base: Int32 = 4;
    show(base);
}
"#,
        &[greet(
            r#"pub fun show(value: Int32) {
    println(value);
}
"#,
        )],
    )
    .unwrap();
    assert!(output.contains("NATIVE_PRINT\tbase\tInt32\t1"), "{output}");
}

#[test]
fn inlines_methods_with_the_receiver_as_this() {
    let output = eval_with(
        r#"
import "greet://hello";

init {
    into greeter = Greeter();
    greeter.present();
}
"#,
        &[greet(
            r#"pub Greeter() {
    #[link("c", "src/ffi/greeter.c", "greeter_new")]
    pub c fun new(): Greeter;

    #[link("c", "src/ffi/greeter.c", "greeter_attach")]
    pub c fun attach(): Void;

    pub fun present() {
        this.attach();
    }
}
"#,
        )],
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
