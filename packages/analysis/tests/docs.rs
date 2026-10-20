mod common;

use analysis::DocComments;
use common::{position_of, session};

const URI: &str = "file:///project/src/main/docs.into";

#[test]
fn a_run_of_doc_comments_documents_the_declaration_below() {
    let docs = DocComments::new("/// First.\n///Second.\npub fun add() {}\n");
    assert_eq!(docs.for_declaration(3), Some("First.\nSecond."));
}

#[test]
fn a_plain_comment_a_blank_line_or_a_trailing_comment_documents_nothing() {
    let source =
        "// plain\nfun a() {}\n\n/// apart\n\nfun b() {}\ninto x = 1; /// trailing\nfun c() {}\n";
    let docs = DocComments::new(source);
    assert_eq!(docs.for_declaration(2), None);
    assert_eq!(docs.for_declaration(6), None);
    assert_eq!(docs.for_declaration(8), None);
}

#[test]
fn attributes_between_the_comment_and_the_declaration_are_skipped() {
    let docs = DocComments::new("/// Runs.\n#[test]\npub fun runs() {}\n");
    assert_eq!(docs.for_declaration(3), Some("Runs."));
}

#[test]
fn hovering_a_call_shows_the_signature_and_the_documentation() {
    let source = "/// Adds two numbers.\n/// Returns the first one.\npub fun add(left: Int32, right: Int32): Int32 {\n    return left;\n}\n\ninit {\n    add(1, 2);\n}\n";
    let mut editor = session();
    editor.open(URI, source);
    let hover = editor
        .hover(URI, position_of(source, "add(1", 0))
        .expect("a hover");
    assert_eq!(
        hover.signature,
        "pub fun add(left: Int32, right: Int32): Int32"
    );
    assert_eq!(
        hover.documentation.unwrap().markdown,
        "Adds two numbers.\nReturns the first one."
    );
    assert_eq!(hover.range.start, position_of(source, "add(1", 0));
}

#[test]
fn a_declaration_without_a_doc_comment_has_no_documentation() {
    let source = "pub fun bare() {}\ninit {\n    bare();\n}\n";
    let mut editor = session();
    editor.open(URI, source);
    let hover = editor
        .hover(URI, position_of(source, "bare();", 0))
        .unwrap();
    assert_eq!(hover.documentation, None);
}

#[test]
fn hovering_nothing_gives_nothing() {
    let source = "init {\n    into x = 1;\n}\n";
    let mut editor = session();
    editor.open(URI, source);
    assert!(editor.hover(URI, position_of(source, "{", 0)).is_none());
}
