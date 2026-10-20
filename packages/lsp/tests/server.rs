mod common;

use common::{Log, URI, server};
use ipc::Request;
use lsp_types::{
    CompletionItemKind, CompletionResponse, CompletionTextEdit, DocumentSymbolResponse,
    GotoDefinitionResponse, Hover, HoverContents, InsertTextFormat, NumberOrString,
    PublishDiagnosticsParams, TextEdit,
};
use serde_json::{Value, json};

fn open(server: &mut lsp::Server<ipc::MemoryTransport>, text: &str) -> Vec<(String, Value)> {
    server.notification(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": URI, "languageId": "into", "version": 1, "text": text}}),
    )
}

fn at(line: u32, character: u32) -> Value {
    json!({"textDocument": {"uri": URI}, "position": {"line": line, "character": character}})
}

#[test]
fn opening_a_document_publishes_the_diagnostics_of_the_analyzer() {
    let mut server = server(&Log::default());
    let sent = open(&mut server, "😀ab\n");
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "textDocument/publishDiagnostics");
    let params: PublishDiagnosticsParams = serde_json::from_value(sent[0].1.clone()).unwrap();
    assert_eq!(params.version, Some(1));
    let diagnostic = &params.diagnostics[0];
    assert_eq!(
        diagnostic.code,
        Some(NumberOrString::String("0x1234".into()))
    );
    assert_eq!(diagnostic.source.as_deref(), Some("intolang"));
    assert_eq!(diagnostic.message, "bad\ntry again");
    assert_eq!(diagnostic.range.start.character, 2);
    assert_eq!(diagnostic.range.end.character, 3);
}

#[test]
fn a_change_publishes_again_and_a_close_clears() {
    let mut server = server(&Log::default());
    open(&mut server, "ab\n");
    let changed = server.notification(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 2}, "contentChanges": [{"text": "abc\n"}]}),
    );
    let params: PublishDiagnosticsParams = serde_json::from_value(changed[0].1.clone()).unwrap();
    assert_eq!(params.version, Some(2));
    assert_eq!(params.diagnostics.len(), 1);
    let closed = server.notification(
        "textDocument/didClose",
        json!({"textDocument": {"uri": URI}}),
    );
    let params: PublishDiagnosticsParams = serde_json::from_value(closed[0].1.clone()).unwrap();
    assert!(params.diagnostics.is_empty());
}

#[test]
fn hover_is_markdown_with_the_signature_and_the_documentation() {
    let mut server = server(&Log::default());
    open(&mut server, "😀ab\n");
    let value = server.request("textDocument/hover", at(0, 2)).unwrap();
    let hover: Hover = serde_json::from_value(value).unwrap();
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("markup expected");
    };
    assert_eq!(markup.value, "```into\npub fun a()\n```\n\nDoes a.");
    let range = hover.range.unwrap();
    assert_eq!((range.start.character, range.end.character), (2, 3));
}

#[test]
fn the_position_goes_to_the_analyzer_in_characters() {
    let log = Log::default();
    let mut server = server(&log);
    open(&mut server, "😀ab\n");
    server.request("textDocument/hover", at(0, 3)).unwrap();
    let requests = log.requests.lock().unwrap();
    let Some(Request::Hover(query)) = requests.last() else {
        panic!("a hover request expected");
    };
    assert_eq!((query.position.line, query.position.column), (0, 2));
}

#[test]
fn nothing_to_hover_is_null() {
    let mut server = server(&Log::default());
    open(&mut server, "a\n\n\n\n\n\n");
    assert_eq!(
        server.request("textDocument/hover", at(5, 0)).unwrap(),
        Value::Null
    );
}

#[test]
fn definition_answers_locations() {
    let mut server = server(&Log::default());
    open(&mut server, "😀ab\n");
    let value = server.request("textDocument/definition", at(0, 0)).unwrap();
    let GotoDefinitionResponse::Array(locations) = serde_json::from_value(value).unwrap() else {
        panic!("an array expected");
    };
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].uri.as_str(), URI);
    assert_eq!(locations[0].range.start.character, 2);
}

#[test]
fn symbols_keep_their_containers() {
    let mut server = server(&Log::default());
    open(&mut server, "Box\n");
    let value = server
        .request(
            "textDocument/documentSymbol",
            json!({"textDocument": {"uri": URI}}),
        )
        .unwrap();
    let DocumentSymbolResponse::Flat(symbols) = serde_json::from_value(value).unwrap() else {
        panic!("a flat list expected");
    };
    assert_eq!(symbols[0].name, "a");
    assert_eq!(symbols[0].container_name.as_deref(), Some("Box"));
    assert_eq!(symbols[0].kind, lsp_types::SymbolKind::METHOD);
    assert_eq!(symbols[1].kind, lsp_types::SymbolKind::CLASS);
}

#[test]
fn formatting_answers_text_edits() {
    let mut server = server(&Log::default());
    open(&mut server, "Box\n");
    let value = server
        .request(
            "textDocument/formatting",
            json!({"textDocument": {"uri": URI}, "options": {"tabSize": 4, "insertSpaces": true}}),
        )
        .unwrap();
    let edits: Vec<TextEdit> = serde_json::from_value(value).unwrap();
    assert_eq!(edits[0].new_text, "formatted");
}

#[test]
fn completion_answers_edits_that_replace_the_prefix() {
    let mut server = server(&Log::default());
    open(&mut server, "😀ab\n");
    let value = server.request("textDocument/completion", at(0, 2)).unwrap();
    let CompletionResponse::Array(items) = serde_json::from_value(value).unwrap() else {
        panic!("an array expected");
    };
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].label, "alpha");
    assert_eq!(items[0].kind, Some(CompletionItemKind::FUNCTION));
    assert_eq!(
        items[0].insert_text_format,
        Some(InsertTextFormat::PLAIN_TEXT)
    );
    let Some(CompletionTextEdit::Edit(edit)) = &items[0].text_edit else {
        panic!("a text edit expected");
    };
    assert_eq!(edit.new_text, "alpha");
    assert_eq!(edit.range.start.character, 0);
    assert_eq!(edit.range.end.character, 2);
}

#[test]
fn a_folder_asks_for_the_next_suggestions_and_a_snippet_says_so() {
    let mut server = server(&Log::default());
    open(&mut server, "ab\n");
    let value = server.request("textDocument/completion", at(0, 0)).unwrap();
    let CompletionResponse::Array(items) = serde_json::from_value(value).unwrap() else {
        panic!("an array expected");
    };
    assert_eq!(items[1].kind, Some(CompletionItemKind::FOLDER));
    assert!(items[1].command.is_some());
    assert!(items[0].command.is_none());
    assert_eq!(items[2].insert_text_format, Some(InsertTextFormat::SNIPPET));
}

#[test]
fn an_unknown_method_and_bad_parameters_are_errors() {
    let mut server = server(&Log::default());
    let error = server
        .request("textDocument/rename", json!({}))
        .unwrap_err();
    assert_eq!(error.code, lsp_server::ErrorCode::MethodNotFound as i32);
    let error = server
        .request("textDocument/hover", json!({"nonsense": 1}))
        .unwrap_err();
    assert_eq!(error.code, lsp_server::ErrorCode::InvalidParams as i32);
}

#[test]
fn the_capabilities_name_what_the_server_answers() {
    let capabilities = lsp::capabilities();
    assert!(capabilities.hover_provider.is_some());
    let completion = capabilities.completion_provider.unwrap();
    assert_eq!(
        completion.trigger_characters.unwrap(),
        [".", "/", ":", "\""]
    );
    assert!(capabilities.definition_provider.is_some());
    assert!(capabilities.document_symbol_provider.is_some());
    assert!(capabilities.document_formatting_provider.is_some());
}
