#![cfg(feature = "run")]

use testing::eval_run;

#[test]
fn runs_a_compiled_program() {
    let source = "import \"intolang://builtin/std\";\ninit {\n    into index: Int32 = 0;\n    while (index < 3) {\n        println(index);\n        index = index + 1;\n    }\n}\n";
    let execution = eval_run(source).unwrap();
    assert!(execution.success, "{}", execution.stderr);
    assert_eq!(execution.stdout, "0\n1\n2\n");
}

#[test]
fn prints_compile_time_text() {
    let source = "import \"intolang://builtin/std\";\ninit {\n    println(\"hello\");\n}\n";
    assert_eq!(eval_run(source).unwrap().stdout, "hello\n");
}

#[test]
fn reports_lowering_errors_before_compiling() {
    let error = eval_run("init {\n    into value: Int32 = \"x\";\n}\n").unwrap_err();
    assert!(error.to_string().contains("Type mismatch"), "{error}");
}
