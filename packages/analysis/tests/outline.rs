mod common;

use common::session;
use ipc::SymbolKind;

const URI: &str = "file:///project/src/main/outline.into";

#[test]
fn the_symbols_list_the_declarations_with_their_containers() {
    let source = "pub Counter() {\n    pub into: Int32 value = 0;\n    pub fun bump() {\n        into inner = 1;\n    }\n}\n\nenum Color { Red }\n\npub fun twice(number: Int32) {}\n";
    let mut editor = session();
    editor.open(URI, source);
    let symbols: Vec<(String, SymbolKind, Option<String>)> = editor
        .symbols(URI)
        .into_iter()
        .map(|symbol| (symbol.name, symbol.kind, symbol.container))
        .collect();
    assert_eq!(
        symbols,
        [
            ("Counter".into(), SymbolKind::Class, None),
            ("value".into(), SymbolKind::Field, Some("Counter".into())),
            ("bump".into(), SymbolKind::Function, Some("Counter".into())),
            ("Color".into(), SymbolKind::Enum, None),
            ("Red".into(), SymbolKind::Variant, Some("Color".into())),
            ("twice".into(), SymbolKind::Function, None),
        ]
    );
}

#[test]
fn formatting_replaces_the_whole_text_and_leaves_formatted_text_alone() {
    let mut editor = session();
    editor.open(URI, "init {\ninto x = 1;\n}\n");
    let edits = editor.format(URI);
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].text, "init {\n    into x = 1;\n}\n");
    assert_eq!(edits[0].range.start, ipc::Position::new(0, 0));
    assert_eq!(edits[0].range.end, ipc::Position::new(3, 0));
    editor.change(URI, &edits[0].text);
    assert!(editor.format(URI).is_empty());
}
