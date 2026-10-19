use formatter::format_source;

#[test]
fn formats_thread_bodies_like_blocks() {
    assert_eq!(
        format_source(r#"now into worker=thread{println("hi");};"#),
        r#"now into worker = thread {
    println("hi");
};
"#
    );
}

#[test]
fn separates_await_from_its_operand() {
    assert_eq!(
        format_source("await   worker.run();"),
        r#"await worker.run();
"#
    );
}
