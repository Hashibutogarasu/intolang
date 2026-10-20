#![allow(dead_code)]

use analysis::Analyzer;
use ipc::{
    CompletionItem, Diagnostic, DocumentChange, DocumentClose, DocumentOpen, DocumentQuery,
    Handler, Hover, InitializeParams, Location, Notification, PROTOCOL_VERSION, Position, Reply,
    Request, Symbol, TextEdit,
};

pub struct Session {
    analyzer: Analyzer,
    version: i64,
}

pub fn session() -> Session {
    session_with(None)
}

pub fn session_with(packages_dir: Option<&str>) -> Session {
    let mut analyzer = Analyzer::new();
    analyzer
        .handle(Request::Initialize(InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            root_uri: None,
            packages_dir: packages_dir.map(str::to_owned),
        }))
        .unwrap();
    Session {
        analyzer,
        version: 0,
    }
}

pub fn position_of(text: &str, needle: &str, occurrence: usize) -> Position {
    let byte = text
        .match_indices(needle)
        .nth(occurrence)
        .unwrap_or_else(|| panic!("`{needle}` occurs fewer than {} times", occurrence + 1))
        .0;
    let before = &text[..byte];
    let line = before.matches('\n').count();
    let column = before.rsplit('\n').next().unwrap().chars().count();
    Position::new(line as u32, column as u32)
}

fn diagnostics(notifications: Vec<Notification>) -> Vec<Diagnostic> {
    match notifications.into_iter().next() {
        Some(Notification::Diagnostics { diagnostics, .. }) => diagnostics,
        other => panic!("expected diagnostics, got {other:?}"),
    }
}

impl Session {
    pub fn open(&mut self, uri: &str, text: &str) -> Vec<Diagnostic> {
        self.version += 1;
        diagnostics(
            self.analyzer
                .notify(Notification::DocumentOpen(DocumentOpen {
                    uri: uri.to_owned(),
                    version: self.version,
                    text: text.to_owned(),
                })),
        )
    }

    pub fn change(&mut self, uri: &str, text: &str) -> Vec<Diagnostic> {
        self.version += 1;
        diagnostics(
            self.analyzer
                .notify(Notification::DocumentChange(DocumentChange {
                    uri: uri.to_owned(),
                    version: self.version,
                    text: text.to_owned(),
                })),
        )
    }

    pub fn close(&mut self, uri: &str) -> Vec<Diagnostic> {
        diagnostics(
            self.analyzer
                .notify(Notification::DocumentClose(DocumentClose {
                    uri: uri.to_owned(),
                })),
        )
    }

    fn query(uri: &str, position: Position) -> DocumentQuery {
        DocumentQuery {
            uri: uri.to_owned(),
            position,
        }
    }

    pub fn hover(&mut self, uri: &str, position: Position) -> Option<Hover> {
        match self
            .analyzer
            .handle(Request::Hover(Self::query(uri, position)))
        {
            Ok(Reply::Hover(hover)) => hover,
            other => panic!("unexpected {other:?}"),
        }
    }

    pub fn definition(&mut self, uri: &str, position: Position) -> Vec<Location> {
        match self
            .analyzer
            .handle(Request::Definition(Self::query(uri, position)))
        {
            Ok(Reply::Definition(locations)) => locations,
            other => panic!("unexpected {other:?}"),
        }
    }

    pub fn symbols(&mut self, uri: &str) -> Vec<Symbol> {
        match self.analyzer.handle(Request::DocumentSymbol(Self::query(
            uri,
            Position::default(),
        ))) {
            Ok(Reply::DocumentSymbol(symbols)) => symbols,
            other => panic!("unexpected {other:?}"),
        }
    }

    pub fn format(&mut self, uri: &str) -> Vec<TextEdit> {
        match self
            .analyzer
            .handle(Request::Format(Self::query(uri, Position::default())))
        {
            Ok(Reply::Format(edits)) => edits,
            other => panic!("unexpected {other:?}"),
        }
    }

    pub fn complete(&mut self, uri: &str, position: Position) -> Vec<CompletionItem> {
        match self
            .analyzer
            .handle(Request::Completion(Self::query(uri, position)))
        {
            Ok(Reply::Completion(items)) => items,
            other => panic!("unexpected {other:?}"),
        }
    }

    pub fn failure(&mut self, request: Request) -> ipc::IpcError {
        self.analyzer.handle(request).unwrap_err()
    }
}
