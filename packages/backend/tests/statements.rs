use testing::eval;
use types::MessageTag;

const STD: &str = "import \"intolang://builtin/std\";\n";

#[test]
fn println_of_a_string_literal_is_compile_time_text() {
    let output = eval(&format!("{STD}init {{\n    println(\"hello\");\n}}\n")).unwrap();
    assert_eq!(output.text(), "hello\n");
}

#[test]
fn print_requires_the_std_import() {
    let error = eval("init {\n    print(\"x\");\n}\n").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Function `print` is not imported"),
        "{error}"
    );
}

#[test]
fn prints_native_variables_and_constants() {
    let output = eval(&format!(
        "{STD}init {{\n    into count: Int32 = 3;\n    println(count);\n    print(2);\n}}\n"
    ))
    .unwrap();
    assert!(output.contains("NATIVE_PRINT\tcount\tInt32\t1"), "{output}");
    assert!(
        output.contains("NATIVE_PRINT_CONST\tInt32\tn:2\t0"),
        "{output}"
    );
}

#[test]
fn string_variables_are_inlined_as_text() {
    let output = eval(&format!(
        "{STD}into message = \"Hello World!\";\ninit {{\n    println(message);\n}}\n"
    ))
    .unwrap();
    assert_eq!(output.text(), "Hello World!\n");
}

#[test]
fn reports_out_of_scope_variables_with_a_backtrace() {
    let error = eval(&format!("{STD}into mem1 = 0;\nprint(mem1);\n")).unwrap_err();
    let text = error.to_string();
    assert!(
        text.contains("Variable `mem1` is out of scope here"),
        "{text}"
    );
    assert!(text.contains("Into language backtrace:"), "{text}");
    assert!(text.contains("module scope at <source>:3"), "{text}");
}

#[test]
fn unloaded_memory_cannot_be_printed() {
    let error = eval(&format!(
        "{STD}now into mem = 5;\nunload mem;\nprint(mem);\n"
    ))
    .unwrap_err();
    assert!(
        error.to_string().contains("Variable `mem` was unloaded"),
        "{error}"
    );
}

#[test]
fn calls_to_unimported_functions_produce_no_output() {
    let output = eval("init {\n    missing();\n    other.thing();\n}\n");
    let output = output.unwrap_err();
    assert!(
        output.to_string().contains("Unknown receiver `other`"),
        "{output}"
    );
    assert_eq!(
        eval("init {\n    missing();\n}\n").unwrap().records().len(),
        2
    );
}

#[test]
fn layout_does_not_change_the_lowered_message() {
    let spread = format!(
        "{STD}init\n{{\n  into\n    count : Int32\n      =\n        1 ;\n  while\n    ( count\n < 3 )\n  {{\n    count=count+1;\n  }}\n}}\n"
    );
    let packed = format!("{STD}init{{into count:Int32=1;while(count<3){{count=count+1;}}}}");
    let lowered = |source: &str| {
        eval(source)
            .unwrap()
            .records()
            .into_iter()
            .filter(|record| record.tag() != MessageTag::Module)
            .collect::<Vec<_>>()
    };
    assert_eq!(lowered(&spread), lowered(&packed));
}

#[test]
fn rejects_nested_function_declarations_with_a_backtrace() {
    let error = eval("pub fun outer() {\n    pub fun inner() {}\n}\n").unwrap_err();
    let text = error.to_string();
    assert!(
        text.contains("Function declarations cannot be nested inside another function"),
        "{text}"
    );
    assert!(text.contains("function body at <source>:2"), "{text}");
}

#[test]
fn validates_enum_declarations() {
    assert!(eval("enum Format { Text, JSON = 2 }\n").is_ok());
    for (source, expected) in [
        ("enum format { A }\n", "must start with an uppercase letter"),
        (
            "enum Format { a }\n",
            "variant names must start with an uppercase letter",
        ),
        ("enum Format { A, A }\n", "Duplicate enum variant `A`"),
        (
            "enum Format { A }\nenum Format { B }\n",
            "Duplicate enum `Format`",
        ),
        ("enum Format { A = 3000000000 }\n", "32-bit integers"),
    ] {
        let error = eval(source).unwrap_err();
        assert!(error.to_string().contains(expected), "{source}: {error}");
    }
}
