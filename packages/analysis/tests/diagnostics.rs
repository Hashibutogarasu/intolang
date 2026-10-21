mod common;

use common::{position_of, session, session_with};
use ipc::{Diagnostic, Position, Request, Severity};

const URI: &str = "file:///project/src/main/diagnostics.into";

fn problems(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.severity != Severity::Hint)
        .collect()
}

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
    let diagnostics = problems(session().open(URI, source));
    assert_eq!(diagnostics.len(), 1);
    let error = &diagnostics[0];
    assert_eq!(error.code, Some(0x100A));
    assert_eq!(error.range.start, Position::new(1, 4));
    assert_eq!(error.range.end.line, 1);
}

#[test]
fn every_syntax_error_is_reported() {
    let source =
        "into first = ;\ninto second = 1;\ninit {\n    into third = ;\n}\ninto fourth = ;\n";
    let diagnostics = session().open(URI, source);
    let lines = diagnostics
        .iter()
        .map(|error| error.range.start.line)
        .collect::<Vec<_>>();
    assert_eq!(lines, vec![0, 3, 5]);
    assert!(diagnostics.iter().all(|error| error.code == Some(0x0001)));
}

#[test]
fn every_error_of_lowering_is_reported() {
    let source = "init {\n    into first: Int32 = \"one\";\n    into second: Int32 = \"two\";\n}\n";
    let diagnostics = problems(session().open(URI, source));
    let lines = diagnostics
        .iter()
        .map(|error| error.range.start.line)
        .collect::<Vec<_>>();
    assert_eq!(lines, vec![1, 2]);
    assert!(diagnostics.iter().all(|error| error.code == Some(0x100A)));
}

#[test]
fn a_failed_declaration_does_not_report_its_uses_as_undefined() {
    let source = "init {\n    into first: Int32 = \"one\";\n    first = 2;\n}\n";
    let diagnostics = session().open(URI, source);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].range.start.line, 1);
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
