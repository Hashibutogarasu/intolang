use formatter::format_source;

#[test]
fn formats_thread_bodies_like_blocks() {
    assert_eq!(
        format_source("now into worker=thread{println(\"hi\");};"),
        "now into worker = thread {\n    println(\"hi\");\n};\n"
    );
}

#[test]
fn separates_await_from_its_operand() {
    assert_eq!(
        format_source("await   worker.run();"),
        "await worker.run();\n"
    );
}
