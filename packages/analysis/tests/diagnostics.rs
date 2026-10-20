mod common;

use common::{position_of, session, session_with};
use ipc::{Position, Request, Severity};

const URI: &str = "file:///project/src/main/diagnostics.into";

#[test]
fn a_syntax_error_covers_the_token_where_the_parser_stopped() {
    let source = "into message = ;\n\ninit {\n}\n";
    let diagnostics = session().open(URI, source);
    assert_eq!(diagnostics.len(), 1);
    let error = &diagnostics[0];
    assert_eq!(error.severity, Severity::Error);
    assert_eq!(error.code, Some(0x0001));
    assert_eq!(error.message, "Expected an expression");
    assert_eq!(error.range.start, position_of(source, ";", 0));
    assert_eq!(error.range.end, Position::new(0, 16));
    assert_eq!(error.found.as_deref(), Some("`;`"));
}

#[test]
fn an_unterminated_string_is_reported_on_its_line() {
    let source = "init {\n    into text = \"open;\n}\n";
    let diagnostics = session().open(URI, source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].range.start.line, 1);
}

#[test]
fn an_error_of_lowering_is_reported_on_the_line_it_happened() {
    let source = "init {\n    into message: Int32 = \"Hello World!\";\n}\n";
    let diagnostics = session().open(URI, source);
    assert_eq!(diagnostics.len(), 1);
    let error = &diagnostics[0];
    assert_eq!(error.code, Some(0x100A));
    assert_eq!(error.range.start, Position::new(1, 4));
    assert_eq!(error.range.end.line, 1);
}

#[test]
fn fixing_the_text_clears_the_diagnostics() {
    let mut editor = session();
    assert_eq!(editor.open(URI, "into message = ;\n").len(), 1);
    assert!(editor.change(URI, "into message = 1;\n").is_empty());
}

#[test]
fn closing_a_document_clears_its_diagnostics() {
    let mut editor = session();
    editor.open(URI, "into message = ;\n");
    assert!(editor.close(URI).is_empty());
}

#[test]
fn an_import_that_names_nothing_is_reported_on_its_string() {
    let source = "import \"nowhere://tools\";\n\ninit {\n}\n";
    let diagnostics = session().open(URI, source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, Some(0x10AA));
    assert_eq!(diagnostics[0].range.start, Position::new(0, 7));
    assert_eq!(diagnostics[0].range.end, Position::new(0, 24));
}

#[test]
fn a_document_that_is_not_open_is_an_error() {
    let mut editor = session_with(None);
    let error = editor.failure(Request::Hover(ipc::DocumentQuery {
        uri: URI.to_owned(),
        position: Position::default(),
    }));
    assert_eq!(error.code, ipc::IpcError::UNKNOWN_DOCUMENT);
}
