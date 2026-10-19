use testing::eval;
use types::MessageTag;

#[test]
fn println_of_a_string_literal_is_compile_time_text() {
    let output = eval(
        r#"import "intolang://builtin/std";
init {
    println("hello");
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.text(),
        r#"hello
"#
    );
}

#[test]
fn print_requires_the_std_import() {
    let error = eval(
        r#"init {
    print("x");
}
"#,
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Function `print` is not imported"),
        "{error}"
    );
}

#[test]
fn prints_native_variables_and_constants() {
    let output = eval(
        r#"import "intolang://builtin/std";
init {
    into count: Int32 = 3;
    println(count);
    print(2);
}
"#,
    )
    .unwrap();
    assert!(output.contains("NATIVE_PRINT\tcount\tInt32\t1"), "{output}");
    assert!(
        output.contains("NATIVE_PRINT_CONST\tInt32\tn:2\t0"),
        "{output}"
    );
}

#[test]
fn string_variables_are_inlined_as_text() {
    let output = eval(
        r#"import "intolang://builtin/std";
into message = "Hello World!";
init {
    println(message);
}
"#,
    )
    .unwrap();
    assert_eq!(
        output.text(),
        r#"Hello World!
"#
    );
}

#[test]
fn reports_out_of_scope_variables_with_a_backtrace() {
    let error = eval(
        r#"import "intolang://builtin/std";
into mem1 = 0;
print(mem1);
"#,
    )
    .unwrap_err();
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
    let error = eval(
        r#"import "intolang://builtin/std";
now into mem = 5;
unload mem;
print(mem);
"#,
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("Variable `mem` was unloaded"),
        "{error}"
    );
}

#[test]
fn calls_to_unimported_functions_produce_no_output() {
    let output = eval(
        r#"init {
    missing();
    other.thing();
}
"#,
    );
    let output = output.unwrap_err();
    assert!(
        output.to_string().contains("Unknown receiver `other`"),
        "{output}"
    );
    assert_eq!(
        eval(
            r#"init {
    missing();
}
"#
        )
        .unwrap()
        .records()
        .iter()
        .filter(|record| record.tag() != types::MessageTag::Ownership)
        .count(),
        2
    );
}

#[test]
fn layout_does_not_change_the_lowered_message() {
    let spread = r#"import "intolang://builtin/std";
init
{
  into
    count : Int32
      =
        1 ;
  while
    ( count
 < 3 )
  {
    count=count+1;
  }
}
"#;
    let packed = r#"import "intolang://builtin/std";
init{into count:Int32=1;while(count<3){count=count+1;}}"#;
    let lowered = |source: &str| {
        eval(source)
            .unwrap()
            .records()
            .into_iter()
            .filter(|record| record.tag() != MessageTag::Module)
            .collect::<Vec<_>>()
    };
    assert_eq!(lowered(spread), lowered(packed));
}

#[test]
fn rejects_nested_function_declarations_with_a_backtrace() {
    let error = eval(
        r#"pub fun outer() {
    pub fun inner() {}
}
"#,
    )
    .unwrap_err();
    let text = error.to_string();
    assert!(
        text.contains("Function declarations cannot be nested inside another function"),
        "{text}"
    );
    assert!(text.contains("function body at <source>:2"), "{text}");
}

#[test]
fn validates_enum_declarations() {
    assert!(
        eval(
            r#"enum Format { Text, JSON = 2 }
"#
        )
        .is_ok()
    );
    for (source, expected) in [
        (
            r#"enum format { A }
"#,
            "must start with an uppercase letter",
        ),
        (
            r#"enum Format { a }
"#,
            "variant names must start with an uppercase letter",
        ),
        (
            r#"enum Format { A, A }
"#,
            "Duplicate enum variant `A`",
        ),
        (
            r#"enum Format { A }
enum Format { B }
"#,
            "Duplicate enum `Format`",
        ),
        (
            r#"enum Format { A = 3000000000 }
"#,
            "32-bit integers",
        ),
    ] {
        let error = eval(source).unwrap_err();
        assert!(error.to_string().contains(expected), "{source}: {error}");
    }
}
